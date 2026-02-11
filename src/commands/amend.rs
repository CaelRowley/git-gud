use clap::Args;
use colored::Colorize;

use crate::git;

#[derive(Args)]
pub struct AmendArgs {
    /// Also stage all changes before amending
    #[arg(short = 'a', long)]
    pub all: bool,

    /// Edit the commit message
    #[arg(short, long)]
    pub edit: bool,
}

pub fn run(args: AmendArgs) -> i32 {
    // Optionally stage all changes
    if args.all {
        println!("Running: {}", "git add -A".bold());
        if git::run(&["add", "-A"]) != 0 {
            return 1;
        }
    }

    // Amend the commit
    let amend_args = if args.edit {
        vec!["commit", "--amend"]
    } else {
        vec!["commit", "--amend", "--no-edit"]
    };

    println!("Running: {}", format!("git {}", amend_args.join(" ")).bold());
    git::run(&amend_args)
}
