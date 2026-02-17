//! Pull LFS files from remote storage

use crate::lfs::{Cache, LfsConfig, Pointer, Scanner};
use crate::lfs::storage::{S3Config, S3Storage, Storage};
use clap::Args;
use colored::Colorize;
use std::path::Path;

#[derive(Args, Debug)]
pub struct PullArgs {
    /// Show what would be pulled without actually pulling
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Include only files matching pattern
    #[arg(short, long)]
    pub include: Option<String>,

    /// Exclude files matching pattern
    #[arg(short, long)]
    pub exclude: Option<String>,
}

/// Pull LFS files from remote storage
pub fn run(args: PullArgs) -> i32 {
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

async fn run_inner(args: PullArgs) -> Result<(), Box<dyn std::error::Error>> {
    let repo = git2::Repository::discover(".")?;
    let repo_root = repo
        .workdir()
        .ok_or("Not a git repository with a working directory")?;

    // Load config
    let config = match LfsConfig::load(repo_root) {
        Ok(c) => c,
        Err(_) => {
            // No config, nothing to pull
            println!("{}", "No LFS configuration found. Run 'gg lfs install' first.".dimmed());
            return Ok(());
        }
    };

    // Initialize storage
    let storage = create_storage(&config).await?;

    // Initialize cache
    let cache = Cache::new()?;

    // Scan for LFS pointer files
    let scanner = Scanner::new(repo_root)?;
    let pointer_files = find_pointer_files(repo_root, &scanner, &args)?;

    if pointer_files.is_empty() {
        println!("{}", "No LFS pointer files found.".dimmed());
        return Ok(());
    }

    println!(
        "{} {} LFS file(s) from {}...",
        if args.dry_run { "Would pull" } else { "Pulling" },
        pointer_files.len(),
        storage.provider_name().cyan()
    );

    let mut downloaded = 0;
    let mut cached = 0;
    let mut errors = 0;

    for (file_path, pointer) in &pointer_files {
        let relative = file_path
            .strip_prefix(repo_root)
            .unwrap_or(file_path);

        let oid = pointer.sha256();

        if args.dry_run {
            println!(
                "  {} {} ({} bytes)",
                "Would download:".cyan(),
                relative.display(),
                pointer.size
            );
            continue;
        }

        // Check cache first
        if let Some(cached_path) = cache.get(oid) {
            // Copy from cache
            std::fs::copy(&cached_path, file_path)?;
            println!(
                "  {} {} (from cache)",
                "Restored:".green(),
                relative.display()
            );
            cached += 1;
            continue;
        }

        // Download from storage
        let temp_path = repo_root.join(".gg").join("tmp").join(oid);
        if let Some(parent) = temp_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        match storage.download(oid, &temp_path).await {
            Ok(result) => {
                // Verify hash
                let downloaded_pointer = Pointer::from_file(&temp_path)?;
                if downloaded_pointer.oid != pointer.oid {
                    eprintln!(
                        "  {} {} - hash mismatch!",
                        "Error:".red(),
                        relative.display()
                    );
                    std::fs::remove_file(&temp_path).ok();
                    errors += 1;
                    continue;
                }

                // Cache the downloaded file
                cache.put_file(oid, &temp_path)?;

                // Move to final location
                std::fs::rename(&temp_path, file_path)?;

                println!(
                    "  {} {} ({} bytes)",
                    "Downloaded:".green(),
                    relative.display(),
                    result.size
                );
                downloaded += 1;
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

    // Clean up temp directory
    let temp_dir = repo_root.join(".gg").join("tmp");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    if args.dry_run {
        println!("\n{}", "Dry run - no files were actually downloaded.".yellow());
    } else {
        println!(
            "\n{}: {} downloaded, {} from cache, {} errors",
            "Done".green().bold(),
            downloaded,
            cached,
            errors
        );
    }

    if errors > 0 {
        Err("Some files failed to download".into())
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

/// Find all pointer files in the repository
fn find_pointer_files(
    repo_root: &Path,
    scanner: &Scanner,
    args: &PullArgs,
) -> Result<Vec<(std::path::PathBuf, Pointer)>, Box<dyn std::error::Error>> {
    let mut pointers = Vec::new();

    let include_pattern = args.include.as_ref().map(|p| glob::Pattern::new(p)).transpose()?;
    let exclude_pattern = args.exclude.as_ref().map(|p| glob::Pattern::new(p)).transpose()?;

    // Scan for files matching LFS patterns
    for file_path in scanner.scan_files()? {
        let relative = file_path
            .strip_prefix(repo_root)
            .unwrap_or(&file_path);

        let relative_str = relative.to_string_lossy();

        // Apply include/exclude filters
        if let Some(ref pattern) = include_pattern {
            if !pattern.matches(&relative_str) {
                continue;
            }
        }

        if let Some(ref pattern) = exclude_pattern {
            if pattern.matches(&relative_str) {
                continue;
            }
        }

        // Check if it's a pointer file
        if let Ok(pointer) = Pointer::parse(&file_path) {
            pointers.push((file_path, pointer));
        }
    }

    Ok(pointers)
}
