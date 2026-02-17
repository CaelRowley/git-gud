//! Push LFS files to remote storage

use crate::lfs::storage;
use crate::lfs::{Cache, LfsConfig, Pointer, Scanner};
use clap::Args;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{BufRead, IsTerminal};
use std::path::Path;

#[derive(Args, Debug)]
pub struct PushArgs {
    /// Show what would be pushed without actually pushing
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Push all LFS files, not just staged ones
    #[arg(short, long)]
    pub all: bool,

    /// Called by the pre-push hook (reads refs from stdin)
    #[arg(long, hide = true)]
    pub pre_push: bool,
}

/// Push LFS files to remote storage
pub fn run(args: PushArgs) -> i32 {
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

    let config = LfsConfig::load(repo_root).map_err(|e| {
        format!("{}\nRun 'gg lfs install' to create a configuration file.", e)
    })?;

    let storage = storage::create_storage(&config).await?;
    let cache = Cache::new()?;
    let scanner = Scanner::new(repo_root)?;

    if scanner.patterns().is_empty() {
        println!("{}", "No LFS patterns defined. Use 'gg lfs track <pattern>' to add files.".yellow());
        return Ok(());
    }

    let files = if args.pre_push {
        get_pre_push_lfs_files(repo_root, &scanner)?
    } else if args.all {
        scanner.scan_files()?
    } else {
        get_staged_lfs_files(&repo, &scanner)?
    };

    if files.is_empty() {
        if !args.pre_push {
            println!("{}", "No LFS files to push.".dimmed());
        }
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

    if !args.dry_run {
        println!(
            "{} {} LFS file(s) to {}...",
            "Pushing", files.len(), storage.provider_name().cyan()
        );
    }

    let mut uploaded = 0;
    let mut skipped = 0;
    let mut errors = 0;

    for file_path in &files {
        let relative = file_path.strip_prefix(repo_root).unwrap_or(file_path);

        if !Pointer::is_pointer_file(file_path) {
            if !args.pre_push {
                let pointer = Pointer::from_file(file_path)?;
                let oid = pointer.sha256();

                if args.dry_run {
                    println!("  {} {} ({} bytes)", "Would upload:".cyan(), relative.display(), pointer.size);
                    continue;
                }

                if storage.exists(oid).await? {
                    cache.put_file(oid, file_path)?;
                    skipped += 1;
                } else {
                    match storage.upload(oid, file_path).await {
                        Ok(_) => {
                            uploaded += 1;
                            cache.put_file(oid, file_path)?;
                        }
                        Err(e) => {
                            if let Some(ref pb) = pb { pb.suspend(|| eprintln!("  {} {} - {}", "Failed:".red(), relative.display(), e)); }
                            errors += 1;
                        }
                    }
                }
            }
            if let Some(ref pb) = pb { pb.inc(1); }
            continue;
        }

        let pointer = Pointer::parse(file_path)?;
        let oid = pointer.sha256();

        if args.dry_run {
            println!("  {} {} ({} bytes)", "Would upload:".cyan(), relative.display(), pointer.size);
            continue;
        }

        if storage.exists(oid).await? {
            skipped += 1;
            if let Some(ref pb) = pb { pb.inc(1); }
            continue;
        }

        if let Some(cached_path) = cache.get(oid) {
            match storage.upload(oid, &cached_path).await {
                Ok(_) => { uploaded += 1; }
                Err(e) => {
                    if let Some(ref pb) = pb { pb.suspend(|| eprintln!("  {} {} - {}", "Failed:".red(), relative.display(), e)); }
                    errors += 1;
                }
            }
        } else {
            skipped += 1;
        }
        if let Some(ref pb) = pb { pb.inc(1); }
    }

    if let Some(pb) = pb { pb.finish_and_clear(); }

    if args.dry_run {
        println!("\n{}", "Dry run - no files were actually uploaded.".yellow());
    } else {
        println!(
            "{}: {} uploaded, {} skipped, {} errors",
            "Done".green().bold(), uploaded, skipped, errors
        );
    }

    if errors > 0 { Err("Some files failed to upload".into()) } else { Ok(()) }
}

/// Get files to push based on pre-push hook stdin
fn get_pre_push_lfs_files(
    repo_root: &Path,
    scanner: &Scanner,
) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let mut files = std::collections::HashSet::new();
    let stdin = std::io::stdin();

    for line in stdin.lock().lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 { continue; }

        let local_sha = parts[1];
        let remote_sha = parts[3];

        if local_sha == "0000000000000000000000000000000000000000" { continue; }

        let diff_args = if remote_sha == "0000000000000000000000000000000000000000" {
            vec!["diff-tree", "-r", "--diff-filter=ACMR", "--name-only", "--root", local_sha]
        } else {
            vec!["diff-tree", "-r", "--diff-filter=ACMR", "--name-only", remote_sha, local_sha]
        };

        let output = std::process::Command::new("git")
            .args(&diff_args)
            .current_dir(repo_root)
            .output()?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for file_line in stdout.lines() {
                let path = Path::new(file_line);
                if scanner.is_lfs_file(path) {
                    let full_path = repo_root.join(path);
                    if full_path.exists() { files.insert(full_path); }
                }
            }
        }
    }

    Ok(files.into_iter().collect())
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
            if full_path.exists() { files.push(full_path); }
        }
    }

    Ok(files)
}

