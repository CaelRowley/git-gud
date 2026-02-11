//! Track/untrack files with LFS

use crate::lfs::Scanner;
use clap::Args;
use colored::Colorize;

#[derive(Args, Debug)]
pub struct TrackArgs {
    /// Pattern to track (e.g., "*.psd", "assets/**")
    pub pattern: String,
}

#[derive(Args, Debug)]
pub struct UntrackArgs {
    /// Pattern to stop tracking
    pub pattern: String,
}

/// Track files matching a pattern
pub fn run(args: TrackArgs) -> i32 {
    match run_inner(args) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            1
        }
    }
}

fn run_inner(args: TrackArgs) -> Result<(), Box<dyn std::error::Error>> {
    let repo = git2::Repository::discover(".")?;
    let repo_root = repo
        .workdir()
        .ok_or("Not a git repository with a working directory")?;

    let mut scanner = Scanner::new(repo_root)?;
    scanner.add_pattern(&args.pattern)?;

    println!(
        "{} \"{}\" {}",
        "Tracking".green(),
        args.pattern.cyan(),
        "with LFS".green()
    );

    // Stage .gitattributes
    let gitattributes = repo_root.join(".gitattributes");
    if gitattributes.exists() {
        crate::git::run(&["add", ".gitattributes"]);
        println!(
            "{}",
            "Staged .gitattributes - commit to save the tracking pattern".dimmed()
        );
    }

    Ok(())
}

/// Stop tracking files matching a pattern
pub fn run_untrack(args: UntrackArgs) -> i32 {
    match run_untrack_inner(args) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            1
        }
    }
}

fn run_untrack_inner(args: UntrackArgs) -> Result<(), Box<dyn std::error::Error>> {
    let repo = git2::Repository::discover(".")?;
    let repo_root = repo
        .workdir()
        .ok_or("Not a git repository with a working directory")?;

    let mut scanner = Scanner::new(repo_root)?;
    let removed = scanner.remove_pattern(&args.pattern)?;

    if removed {
        println!(
            "{} \"{}\"",
            "Untracked".green(),
            args.pattern.cyan()
        );

        // Stage .gitattributes
        crate::git::run(&["add", ".gitattributes"]);
        println!(
            "{}",
            "Staged .gitattributes - commit to save the change".dimmed()
        );
    } else {
        println!(
            "{} \"{}\" {}",
            "Pattern".yellow(),
            args.pattern,
            "was not being tracked".yellow()
        );
    }

    Ok(())
}
