use clap::Args;
use colored::Colorize;

use crate::git;

#[derive(Args)]
pub struct UndoArgs {
    /// Number of commits to undo (default: 1)
    #[arg(default_value = "1")]
    pub count: u32,

    /// Discard changes entirely (hard reset)
    #[arg(long)]
    pub hard: bool,
}

pub fn run(args: UndoArgs) -> i32 {
    let reset_ref = format!("HEAD~{}", args.count);

    let reset_args = if args.hard {
        vec!["reset", "--hard", &reset_ref]
    } else {
        // Soft reset: keeps changes staged
        vec!["reset", "--soft", &reset_ref]
    };

    println!("Running: {}", format!("git {}", reset_args.join(" ")).bold());
    git::run(&reset_args)
}
