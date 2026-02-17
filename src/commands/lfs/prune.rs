//! Prune old LFS objects from the local cache

use crate::lfs::Cache;
use clap::Args;
use colored::Colorize;

#[derive(Args, Debug)]
pub struct PruneArgs {
    /// Remove objects not accessed in this many days (default: 30)
    #[arg(short, long, default_value = "30")]
    pub days: u32,

    /// Show what would be pruned without actually removing
    #[arg(short = 'n', long)]
    pub dry_run: bool,
}

/// Prune old LFS cache objects
pub fn run(args: PruneArgs) -> i32 {
    match run_inner(args) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            1
        }
    }
}

fn run_inner(args: PruneArgs) -> Result<(), Box<dyn std::error::Error>> {
    let cache = Cache::new()?;

    let count = cache.count()?;
    let size = cache.size()?;

    if count == 0 {
        println!("{}", "Cache is empty, nothing to prune.".dimmed());
        return Ok(());
    }

    println!(
        "Cache: {} object(s), {} total",
        count,
        format_bytes(size)
    );

    if args.dry_run {
        // For dry run, just report what would happen
        println!(
            "\n{} Would prune objects not accessed in {} day(s).",
            "Dry run:".cyan(),
            args.days
        );
        println!("{}", "No files were actually removed.".yellow());
    } else {
        let pruned = cache.prune(args.days)?;
        let new_size = cache.size()?;

        if pruned == 0 {
            println!(
                "\n{} No objects older than {} day(s).",
                "Done:".green().bold(),
                args.days
            );
        } else {
            println!(
                "\n{}: pruned {} object(s), freed {}",
                "Done".green().bold(),
                pruned,
                format_bytes(size - new_size)
            );
        }
    }

    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
