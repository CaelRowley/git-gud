//! Migrate from standard git-lfs to gg lfs
//!
//! Transfers files from a git-lfs server to gg's S3 storage.
//! The pointer format is identical (both use git-lfs spec v1),
//! so only the storage backend changes.
//!
//! Steps:
//! 1. Verify git-lfs is installed and the repo uses it
//! 2. Fetch all LFS objects into the local git-lfs cache
//! 3. Upload each object from the git-lfs cache to S3
//! 4. Cache in gg's local cache
//! 5. Uninstall git-lfs hooks (optional)

use crate::lfs::storage::{S3Config, S3Storage, Storage};
use crate::lfs::{Cache, LfsConfig, Pointer, Scanner};
use clap::Args;
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Args, Debug)]
pub struct MigrateArgs {
    /// Show what would happen without making changes
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Skip running 'git lfs fetch --all' before migrating
    #[arg(long)]
    pub skip_fetch: bool,

    /// Keep git-lfs installed (don't remove git-lfs hooks/config)
    #[arg(long)]
    pub keep_gitlfs: bool,
}

/// Migrate from git-lfs to gg lfs
pub fn run(args: MigrateArgs) -> i32 {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!(
                "{} Failed to create async runtime: {}",
                "Error:".red().bold(),
                e
            );
            return 1;
        }
    };

    rt.block_on(async {
        match run_inner(args).await {
            Ok(_) => 0,
            Err(e) => {
                eprintln!("{} {}", "Error:".red().bold(), e);
                1
            }
        }
    })
}

async fn run_inner(args: MigrateArgs) -> Result<(), Box<dyn std::error::Error>> {
    let repo = git2::Repository::discover(".")?;
    let repo_root = repo
        .workdir()
        .ok_or("Not a git repository with a working directory")?;

    // Step 1: Verify git-lfs is available
    println!("{}", "Checking git-lfs...".dimmed());
    if !is_gitlfs_installed() {
        return Err("git-lfs is not installed. Install it first: https://git-lfs.com".into());
    }

    // Check that the repo actually uses git-lfs
    let scanner = Scanner::new(repo_root)?;
    let patterns = scanner.patterns();
    if patterns.is_empty() {
        return Err(
            "No LFS patterns found in .gitattributes. Is this repo using git-lfs?".into(),
        );
    }

    println!(
        "  Found {} LFS pattern(s) in .gitattributes",
        patterns.len()
    );

    // Step 2: Load gg lfs config (must have run 'gg lfs install' first)
    let config = LfsConfig::load(repo_root).map_err(|e| {
        format!(
            "{}\nRun 'gg lfs install' first to configure S3 storage.",
            e
        )
    })?;

    let storage = create_storage(&config).await?;

    // Step 3: Fetch all LFS objects from git-lfs server
    if !args.skip_fetch {
        println!("\n{}", "Fetching all objects from git-lfs server...".cyan());
        if args.dry_run {
            println!("  {} Would run: git lfs fetch --all", "Dry run:".cyan());
        } else {
            let status = Command::new("git")
                .args(["lfs", "fetch", "--all"])
                .status()?;
            if !status.success() {
                return Err("'git lfs fetch --all' failed. Ensure you have access to the git-lfs server.".into());
            }
        }
    }

    // Step 4: Find the git-lfs cache directory
    let lfs_objects_dir = repo_root.join(".git").join("lfs").join("objects");
    if !lfs_objects_dir.exists() && !args.dry_run {
        return Err(format!(
            "git-lfs cache not found at {}. Run 'git lfs fetch --all' first.",
            lfs_objects_dir.display()
        )
        .into());
    }

    // Step 5: Find all files tracked by LFS and upload to S3
    let cache = Cache::new()?;
    let files = scanner.scan_files()?;

    // Separate into pointer files and real files
    let mut pointer_files: Vec<(PathBuf, Pointer)> = Vec::new();
    for file_path in &files {
        if let Ok(pointer) = Pointer::parse(file_path) {
            pointer_files.push((file_path.clone(), pointer));
        }
    }

    // Also check for real files (git-lfs smudge may have expanded them)
    let mut real_files: Vec<PathBuf> = Vec::new();
    for file_path in &files {
        if !Pointer::is_pointer_file(file_path) {
            real_files.push(file_path.clone());
        }
    }

    let total = pointer_files.len() + real_files.len();
    if total == 0 {
        println!("{}", "No LFS files found to migrate.".dimmed());
        return Ok(());
    }

    println!(
        "\n{} {} file(s) to {} ({} pointers, {} expanded)...",
        if args.dry_run {
            "Would migrate"
        } else {
            "Migrating"
        },
        total,
        storage.provider_name().cyan(),
        pointer_files.len(),
        real_files.len()
    );

    let mut uploaded = 0;
    let mut skipped = 0;
    let mut errors = 0;

    // Handle pointer files: find real content in git-lfs cache, upload to S3
    for (file_path, pointer) in &pointer_files {
        let relative = file_path.strip_prefix(repo_root).unwrap_or(file_path);
        let oid = pointer.sha256();

        if args.dry_run {
            println!(
                "  {} {} ({} bytes, pointer -> S3)",
                "Would upload:".cyan(),
                relative.display(),
                pointer.size
            );
            continue;
        }

        // Check if already in S3
        if storage.exists(oid).await? {
            println!(
                "  {} {} (already in S3)",
                "Skip:".dimmed(),
                relative.display()
            );
            cache_from_gitlfs(&lfs_objects_dir, oid, &cache);
            skipped += 1;
            continue;
        }

        // Find the real file in git-lfs cache
        let lfs_cached = find_gitlfs_object(&lfs_objects_dir, oid);
        match lfs_cached {
            Some(lfs_path) => {
                match storage.upload(oid, &lfs_path).await {
                    Ok(_) => {
                        cache.put_file(oid, &lfs_path)?;
                        println!(
                            "  {} {} ({} bytes)",
                            "Uploaded:".green(),
                            relative.display(),
                            pointer.size
                        );
                        uploaded += 1;
                    }
                    Err(e) => {
                        eprintln!("  {} {} - {}", "Failed:".red(), relative.display(), e);
                        errors += 1;
                    }
                }
            }
            None => {
                eprintln!(
                    "  {} {} - not found in git-lfs cache (try 'git lfs fetch --all')",
                    "Missing:".red(),
                    relative.display()
                );
                errors += 1;
            }
        }
    }

    // Handle real files (smudge-expanded): upload directly, then replace with pointer
    for file_path in &real_files {
        let relative = file_path.strip_prefix(repo_root).unwrap_or(file_path);
        let pointer = Pointer::from_file(file_path)?;
        let oid = pointer.sha256();

        if args.dry_run {
            println!(
                "  {} {} ({} bytes, real file -> S3 + pointer)",
                "Would convert:".cyan(),
                relative.display(),
                pointer.size
            );
            continue;
        }

        // Upload to S3 if not already there
        if !storage.exists(oid).await? {
            match storage.upload(oid, file_path).await {
                Ok(_) => {
                    println!(
                        "  {} {} ({} bytes)",
                        "Uploaded:".green(),
                        relative.display(),
                        pointer.size
                    );
                }
                Err(e) => {
                    eprintln!("  {} {} - {}", "Failed:".red(), relative.display(), e);
                    errors += 1;
                    continue;
                }
            }
        }

        // Cache and replace with pointer
        cache.put_file(oid, file_path)?;
        pointer.write(file_path)?;
        uploaded += 1;

        println!(
            "  {} {} ({} bytes)",
            "Converted:".green(),
            relative.display(),
            pointer.size
        );
    }

    // Step 6: Uninstall git-lfs (unless --keep-gitlfs)
    if !args.keep_gitlfs && !args.dry_run {
        println!("\n{}", "Removing git-lfs hooks...".cyan());
        let status = Command::new("git")
            .args(["lfs", "uninstall"])
            .status();
        match status {
            Ok(s) if s.success() => println!("  {} git-lfs hooks", "Removed:".green()),
            _ => println!(
                "  {} Could not uninstall git-lfs (you can do this manually)",
                "Warning:".yellow()
            ),
        }
    } else if args.dry_run && !args.keep_gitlfs {
        println!(
            "\n  {} Would run: git lfs uninstall",
            "Dry run:".cyan()
        );
    }

    if args.dry_run {
        println!(
            "\n{}",
            "Dry run - no files were actually migrated.".yellow()
        );
    } else {
        println!(
            "\n{}: {} uploaded, {} skipped, {} errors",
            "Done".green().bold(),
            uploaded,
            skipped,
            errors
        );

        if errors == 0 {
            println!(
                "\n{}",
                "Migration complete! Your files are now stored in S3 via gg lfs."
                    .green()
                    .bold()
            );
            println!(
                "{}",
                "The .gitattributes file still has filter=lfs entries which gg lfs uses.".dimmed()
            );
        }
    }

    if errors > 0 {
        Err("Some files failed to migrate".into())
    } else {
        Ok(())
    }
}

/// Check if git-lfs is installed
fn is_gitlfs_installed() -> bool {
    Command::new("git")
        .args(["lfs", "version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Find a git-lfs cached object by OID
/// git-lfs stores objects at .git/lfs/objects/{oid[0..2]}/{oid[2..4]}/{oid}
fn find_gitlfs_object(lfs_objects_dir: &Path, oid: &str) -> Option<PathBuf> {
    if oid.len() < 4 {
        return None;
    }
    let path = lfs_objects_dir
        .join(&oid[..2])
        .join(&oid[2..4])
        .join(oid);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

/// Try to cache a git-lfs object in gg's cache
fn cache_from_gitlfs(lfs_objects_dir: &Path, oid: &str, cache: &Cache) {
    if let Some(lfs_path) = find_gitlfs_object(lfs_objects_dir, oid) {
        let _ = cache.put_file(oid, &lfs_path);
    }
}

/// Create storage backend from config
async fn create_storage(
    config: &LfsConfig,
) -> Result<Box<dyn Storage>, Box<dyn std::error::Error>> {
    let s3_config = S3Config {
        bucket: config.storage.bucket.clone(),
        region: config.storage.region.clone(),
        prefix: config.storage.prefix.clone(),
        endpoint: config.storage.endpoint.clone(),
    };

    let storage = S3Storage::new(s3_config).await?;
    Ok(Box::new(storage))
}
