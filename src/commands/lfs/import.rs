//! Import existing large files into LFS
//!
//! Bulk-converts all files matching LFS patterns to pointer files,
//! uploading the real content to S3. Use this for initial setup
//! when adopting gg lfs on a repo that has never used any LFS system.

use crate::lfs::storage;
use crate::lfs::{Cache, LfsConfig, Pointer, Scanner};
use clap::Args;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::IsTerminal;
use std::path::Path;

#[derive(Args, Debug)]
pub struct ImportArgs {
    /// Show what would happen without making changes
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Only import files matching glob pattern
    #[arg(short, long)]
    pub include: Option<String>,

    /// Skip files matching glob pattern
    #[arg(short, long)]
    pub exclude: Option<String>,
}

/// Import large files into LFS
pub fn run(args: ImportArgs) -> i32 {
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

async fn run_inner(args: ImportArgs) -> Result<(), Box<dyn std::error::Error>> {
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
    let storage = storage::create_storage(&config).await?;

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

    // Find all files matching LFS patterns, applying include/exclude filters
    let files = find_matching_files(repo_root, &scanner, &args.include, &args.exclude)?;

    if files.is_empty() {
        println!("{}", "No files to import.".dimmed());
        return Ok(());
    }

    let show_progress = !args.dry_run && std::io::stderr().is_terminal();
    let pb = if show_progress {
        let pb = ProgressBar::new(files.len() as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("  {bar:30} {pos}/{len} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_bar()));
        Some(pb)
    } else {
        None
    };

    println!(
        "{} {} file(s) into LFS via {}...",
        if args.dry_run {
            "Would import"
        } else {
            "Importing"
        },
        files.len(),
        storage.provider_name().cyan()
    );

    let mut converted = 0;
    let mut skipped = 0;
    let mut errors = 0;

    for file_path in &files {
        let relative = file_path.strip_prefix(repo_root).unwrap_or(file_path);

        // Skip files already converted to pointers
        if Pointer::is_pointer_file(file_path) {
            skipped += 1;
            if let Some(ref pb) = pb { pb.inc(1); }
            continue;
        }

        let pointer = Pointer::from_file(file_path)?;
        let oid = pointer.sha256();

        if args.dry_run {
            println!(
                "  {} {} ({} bytes)",
                "Would import:".cyan(),
                relative.display(),
                pointer.size
            );
            continue;
        }

        // Upload to storage if not already there
        if !storage.exists(oid).await? {
            match storage.upload(oid, file_path).await {
                Ok(_) => {}
                Err(e) => {
                    if let Some(ref pb) = pb { pb.suspend(|| eprintln!("  {} {} - {}", "Failed:".red(), relative.display(), e)); }
                    errors += 1;
                    if let Some(ref pb) = pb { pb.inc(1); }
                    continue;
                }
            }
        }

        // Cache locally and replace with pointer
        cache.put_file(oid, file_path)?;
        pointer.write(file_path)?;
        converted += 1;

        if let Some(ref pb) = pb { pb.inc(1); }
    }

    if let Some(pb) = pb { pb.finish_and_clear(); }

    if args.dry_run {
        println!(
            "\n{}",
            "Dry run - no files were actually imported.".yellow()
        );
    } else {
        println!(
            "\n{}: {} imported, {} skipped, {} errors",
            "Done".green().bold(),
            converted,
            skipped,
            errors
        );
    }

    if errors > 0 {
        Err("Some files failed to import".into())
    } else {
        Ok(())
    }
}

/// Find all files matching LFS patterns with optional include/exclude filters
fn find_matching_files(
    repo_root: &Path,
    scanner: &Scanner,
    include: &Option<String>,
    exclude: &Option<String>,
) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let include_pattern = include
        .as_ref()
        .map(|p| globset::Glob::new(p).map(|g| g.compile_matcher()))
        .transpose()?;
    let exclude_pattern = exclude
        .as_ref()
        .map(|p| globset::Glob::new(p).map(|g| g.compile_matcher()))
        .transpose()?;

    let mut files = Vec::new();

    for file_path in scanner.scan_files()? {
        let relative = file_path.strip_prefix(repo_root).unwrap_or(&file_path);

        if let Some(ref pattern) = include_pattern {
            if !pattern.is_match(relative) {
                continue;
            }
        }

        if let Some(ref pattern) = exclude_pattern {
            if pattern.is_match(relative) {
                continue;
            }
        }

        files.push(file_path);
    }

    Ok(files)
}
