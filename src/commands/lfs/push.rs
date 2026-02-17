//! Push LFS files to remote storage

use crate::lfs::{Cache, LfsConfig, Pointer, Scanner};
use crate::lfs::storage::{S3Config, S3Storage, Storage};
use clap::Args;
use colored::Colorize;
use std::path::Path;

#[derive(Args, Debug)]
pub struct PushArgs {
    /// Show what would be pushed without actually pushing
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Push all LFS files, not just staged ones
    #[arg(short, long)]
    pub all: bool,
}

/// Push LFS files to remote storage
pub fn run(args: PushArgs) -> i32 {
    // Create tokio runtime for async operations
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("{} Failed to create async runtime: {}", "Error:".red().bold(), e);
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

async fn run_inner(args: PushArgs) -> Result<(), Box<dyn std::error::Error>> {
    let repo = git2::Repository::discover(".")?;
    let repo_root = repo
        .workdir()
        .ok_or("Not a git repository with a working directory")?;

    // Load config
    let config = LfsConfig::load(repo_root).map_err(|e| {
        format!(
            "{}\nRun 'gg lfs install' to create a configuration file.",
            e
        )
    })?;

    // Initialize storage
    let storage = create_storage(&config).await?;

    // Initialize cache
    let cache = Cache::new()?;

    // Scan for LFS files
    let scanner = Scanner::new(repo_root)?;
    let patterns = scanner.patterns();

    if patterns.is_empty() {
        println!(
            "{}",
            "No LFS patterns defined. Use 'gg lfs track <pattern>' to add files.".yellow()
        );
        return Ok(());
    }

    // Find files to push
    let files = if args.all {
        scanner.scan_files()?
    } else {
        // Get staged files that match LFS patterns
        get_staged_lfs_files(&repo, &scanner)?
    };

    if files.is_empty() {
        println!("{}", "No LFS files to push.".dimmed());
        return Ok(());
    }

    println!(
        "{} {} LFS file(s) to {}...",
        if args.dry_run { "Would push" } else { "Pushing" },
        files.len(),
        storage.provider_name().cyan()
    );

    let mut uploaded = 0;
    let mut skipped = 0;
    let mut errors = 0;

    for file_path in &files {
        let relative = file_path
            .strip_prefix(repo_root)
            .unwrap_or(file_path);

        // Skip if already a pointer file
        if Pointer::is_pointer_file(file_path) {
            println!("  {} {} (already a pointer)", "Skip:".dimmed(), relative.display());
            skipped += 1;
            continue;
        }

        // Create pointer from file
        let pointer = Pointer::from_file(file_path)?;
        let oid = pointer.sha256();

        if args.dry_run {
            println!(
                "  {} {} ({} bytes)",
                "Would upload:".cyan(),
                relative.display(),
                pointer.size
            );
            continue;
        }

        // Check if already in storage
        if storage.exists(oid).await? {
            println!(
                "  {} {} (already in storage)",
                "Skip:".dimmed(),
                relative.display()
            );

            // Still need to replace with pointer if it's not already
            replace_with_pointer(file_path, &pointer, &cache)?;
            skipped += 1;
            continue;
        }

        // Upload to storage
        match storage.upload(oid, file_path).await {
            Ok(result) => {
                if result.uploaded {
                    println!(
                        "  {} {} ({} bytes)",
                        "Uploaded:".green(),
                        relative.display(),
                        result.size
                    );
                    uploaded += 1;
                }

                // Cache the file locally and replace with pointer
                cache.put_file(oid, file_path)?;
                replace_with_pointer(file_path, &pointer, &cache)?;
            }
            Err(e) => {
                eprintln!(
                    "  {} {} - {}",
                    "Failed:".red(),
                    relative.display(),
                    e
                );
                errors += 1;
            }
        }
    }

    if args.dry_run {
        println!("\n{}", "Dry run - no files were actually uploaded.".yellow());
    } else {
        println!(
            "\n{}: {} uploaded, {} skipped, {} errors",
            "Done".green().bold(),
            uploaded,
            skipped,
            errors
        );
    }

    if errors > 0 {
        Err("Some files failed to upload".into())
    } else {
        Ok(())
    }
}

/// Create storage backend from config
async fn create_storage(config: &LfsConfig) -> Result<Box<dyn Storage>, Box<dyn std::error::Error>> {
    let s3_config = S3Config {
        bucket: config.storage.bucket.clone(),
        region: config.storage.region.clone(),
        prefix: config.storage.prefix.clone(),
        endpoint: config.storage.endpoint.clone(),
        credentials: config.storage.credentials.as_ref().map(|c| {
            crate::lfs::storage::s3::S3Credentials {
                access_key_id: c.access_key_id.clone(),
                secret_access_key: c.secret_access_key.clone(),
            }
        }),
    };

    let storage = S3Storage::new(s3_config).await?;
    Ok(Box::new(storage))
}

/// Get staged files that match LFS patterns
fn get_staged_lfs_files(
    repo: &git2::Repository,
    scanner: &Scanner,
) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    let repo_root = repo.workdir().ok_or("No working directory")?;

    let index = repo.index()?;

    for entry in index.iter() {
        let path_str = String::from_utf8_lossy(&entry.path);
        let path = Path::new(path_str.as_ref());

        if scanner.is_lfs_file(path) {
            let full_path = repo_root.join(path);
            if full_path.exists() {
                files.push(full_path);
            }
        }
    }

    Ok(files)
}

/// Replace a file with its pointer
fn replace_with_pointer(
    file_path: &Path,
    pointer: &Pointer,
    _cache: &Cache,
) -> Result<(), Box<dyn std::error::Error>> {
    // Write pointer file
    pointer.write(file_path)?;
    Ok(())
}
