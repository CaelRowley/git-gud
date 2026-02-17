//! Pull LFS files from remote storage

use crate::lfs::storage;
use crate::lfs::{Cache, LfsConfig, Pointer, Scanner};
use clap::Args;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::IsTerminal;
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

    /// Called by the post-checkout hook (old-ref new-ref flag)
    #[arg(long, hide = true, num_args = 3, value_names = &["OLD_REF", "NEW_REF", "FLAG"])]
    pub post_checkout: Option<Vec<String>>,
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
    let storage = storage::create_storage(&config).await?;

    // Initialize cache
    let cache = Cache::new()?;

    // Scan for LFS pointer files
    let scanner = Scanner::new(repo_root)?;

    let pointer_files = if let Some(ref checkout_args) = args.post_checkout {
        // Post-checkout mode: only pull files that changed between old and new refs
        find_post_checkout_pointer_files(repo_root, &scanner, checkout_args)?
    } else {
        find_pointer_files(repo_root, &scanner, &args)?
    };

    if pointer_files.is_empty() {
        if args.post_checkout.is_none() {
            println!("{}", "No LFS pointer files found.".dimmed());
        }
        return Ok(());
    }

    let show_progress = !args.dry_run && std::io::stderr().is_terminal();
    let pb = if show_progress {
        let pb = ProgressBar::new(pointer_files.len() as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("  {bar:30} {pos}/{len} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_bar()));
        Some(pb)
    } else {
        None
    };

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
            cached += 1;
            if let Some(ref pb) = pb { pb.inc(1); }
            continue;
        }

        // Download from storage
        let temp_path = repo_root.join(".gg").join("tmp").join(oid);
        if let Some(parent) = temp_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        match storage.download(oid, &temp_path).await {
            Ok(_result) => {
                // Verify hash
                let downloaded_pointer = Pointer::from_file(&temp_path)?;
                if downloaded_pointer.oid != pointer.oid {
                    if let Some(ref pb) = pb { pb.suspend(|| eprintln!("  {} {} - hash mismatch!", "Error:".red(), relative.display())); }
                    std::fs::remove_file(&temp_path).ok();
                    errors += 1;
                    if let Some(ref pb) = pb { pb.inc(1); }
                    continue;
                }

                // Cache the downloaded file
                cache.put_file(oid, &temp_path)?;

                // Move to final location
                std::fs::rename(&temp_path, file_path)?;

                downloaded += 1;
            }
            Err(e) => {
                if let Some(ref pb) = pb { pb.suspend(|| eprintln!("  {} {} - {}", "Failed:".red(), relative.display(), e)); }
                errors += 1;
            }
        }
        if let Some(ref pb) = pb { pb.inc(1); }
    }

    if let Some(pb) = pb { pb.finish_and_clear(); }

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

/// Find pointer files that changed between two refs (for post-checkout hook).
/// checkout_args: [old_ref, new_ref, flag]
fn find_post_checkout_pointer_files(
    repo_root: &Path,
    scanner: &Scanner,
    checkout_args: &[String],
) -> Result<Vec<(std::path::PathBuf, Pointer)>, Box<dyn std::error::Error>> {
    let old_ref = &checkout_args[0];
    let new_ref = &checkout_args[1];

    // Get files that changed between old and new refs
    let output = std::process::Command::new("git")
        .args(["diff-tree", "-r", "--name-only", old_ref, new_ref])
        .current_dir(repo_root)
        .output()?;

    let mut pointers = Vec::new();

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let path = Path::new(line);
            if scanner.is_lfs_file(path) {
                let full_path = repo_root.join(path);
                if full_path.exists() {
                    if let Ok(pointer) = Pointer::parse(&full_path) {
                        pointers.push((full_path, pointer));
                    }
                }
            }
        }
    }

    Ok(pointers)
}
