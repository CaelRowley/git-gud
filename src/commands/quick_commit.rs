use clap::Args;
use colored::Colorize;

use crate::git;

#[derive(Args)]
pub struct QuickCommitArgs {
    /// Commit message
    pub message: String,

    /// Also push after committing
    #[arg(short, long)]
    pub push: bool,

    /// Add all changes (including untracked files)
    #[arg(short = 'A', long)]
    pub all: bool,
}

pub fn run(args: QuickCommitArgs) -> i32 {
    // Stage changes
    let add_args = if args.all { "-A" } else { "-u" };
    println!("Running: {}", format!("git add {}", add_args).bold());
    if git::run(&["add", add_args]) != 0 {
        return 1;
    }

    // Commit
    println!("Running: {}", format!("git commit -m \"{}\"", args.message).bold());
    if git::run(&["commit", "-m", &args.message]) != 0 {
        return 1;
    }

    // Optionally push
    if args.push {
        println!();
        println!("Running: {}", "git push".bold());
        return git::run(&["push"]);
    }

    0
}
