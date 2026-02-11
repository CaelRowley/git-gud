//! LFS status command

use crate::lfs::{Cache, LfsConfig, Pointer, Scanner};
use clap::Args;
use colored::Colorize;
use std::path::Path;

#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Show detailed information
    #[arg(short, long)]
    pub verbose: bool,
}

/// Show LFS status
pub fn run(args: StatusArgs) -> i32 {
    match run_inner(args) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            1
        }
    }
}

fn run_inner(args: StatusArgs) -> Result<(), Box<dyn std::error::Error>> {
    let repo = git2::Repository::discover(".")?;
    let repo_root = repo
        .workdir()
        .ok_or("Not a git repository with a working directory")?;

    // Check for config
    let config_exists = LfsConfig::exists(repo_root);

    println!("{}", "gg-lfs Status".bold());
    println!("{}", "=".repeat(40));

    // Configuration status
    println!("\n{}", "Configuration:".cyan().bold());
    if config_exists {
        let config = LfsConfig::load(repo_root)?;
        println!(
            "  Provider: {}",
            format!("{:?}", config.storage.provider).green()
        );
        println!("  Bucket:   {}", config.storage.bucket);
        println!("  Region:   {}", config.storage.region);
        if let Some(prefix) = &config.storage.prefix {
            println!("  Prefix:   {}", prefix);
        }
        if let Some(endpoint) = &config.storage.endpoint {
            println!("  Endpoint: {}", endpoint);
        }
    } else {
        println!(
            "  {}",
            "Not configured. Run 'gg lfs install' to set up.".yellow()
        );
    }

    // Patterns
    println!("\n{}", "Tracked Patterns:".cyan().bold());
    let scanner = Scanner::new(repo_root)?;
    let patterns = scanner.patterns();

    if patterns.is_empty() {
        println!(
            "  {}",
            "No patterns. Use 'gg lfs track <pattern>' to add.".dimmed()
        );
    } else {
        for pattern in patterns {
            println!("  {}", pattern.pattern);
        }
    }

    // Files
    println!("\n{}", "LFS Files:".cyan().bold());
    let files = scanner.scan_files()?;

    if files.is_empty() {
        println!("  {}", "No files matching LFS patterns.".dimmed());
    } else {
        let mut pointers = 0;
        let mut actual_files = 0;
        let mut total_size: u64 = 0;

        for file_path in &files {
            let relative = file_path
                .strip_prefix(repo_root)
                .unwrap_or(file_path);

            if Pointer::is_pointer_file(file_path) {
                if args.verbose {
                    let pointer = Pointer::parse(file_path)?;
                    println!(
                        "  {} {} ({} bytes, pointer)",
                        "→".dimmed(),
                        relative.display(),
                        pointer.size
                    );
                }
                pointers += 1;
            } else {
                let size = std::fs::metadata(file_path)?.len();
                total_size += size;

                if args.verbose {
                    println!(
                        "  {} {} ({} bytes)",
                        "●".green(),
                        relative.display(),
                        size
                    );
                }
                actual_files += 1;
            }
        }

        if !args.verbose {
            println!("  {} file(s) as pointers", pointers);
            println!(
                "  {} file(s) need upload ({} bytes)",
                actual_files,
                format_size(total_size)
            );
        } else {
            println!();
            println!("  {} pointers, {} actual files", pointers, actual_files);
        }
    }

    // Cache status
    println!("\n{}", "Local Cache:".cyan().bold());
    match Cache::new() {
        Ok(cache) => {
            let count = cache.count().unwrap_or(0);
            let size = cache.size().unwrap_or(0);
            println!("  {} objects ({})", count, format_size(size));
        }
        Err(_) => {
            println!("  {}", "Cache not available".dimmed());
        }
    }

    // Hooks status
    println!("\n{}", "Git Hooks:".cyan().bold());
    let hooks_dir = repo_root.join(".git").join("hooks");
    let hooks = ["pre-push", "post-checkout", "post-merge"];

    for hook in hooks {
        let hook_path = hooks_dir.join(hook);
        let status = if hook_path.exists() {
            if is_lfs_hook(&hook_path) {
                "installed".green().to_string()
            } else {
                "exists (not gg-lfs)".yellow().to_string()
            }
        } else {
            "not installed".dimmed().to_string()
        };
        println!("  {}: {}", hook, status);
    }

    Ok(())
}

/// Check if a hook file is a gg-lfs hook
fn is_lfs_hook(path: &Path) -> bool {
    if let Ok(content) = std::fs::read_to_string(path) {
        content.contains("gg-lfs") || content.contains("gg lfs")
    } else {
        false
    }
}

/// Format bytes as human-readable size
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
