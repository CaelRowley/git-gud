use clap::Args;

use crate::git;

#[derive(Args)]
pub struct TodayArgs {
    /// Show all authors, not just yours
    #[arg(short, long)]
    pub all: bool,
}

pub fn run(args: TodayArgs) -> i32 {
    let mut log_args = vec![
        "log",
        "--oneline",
        "--since=midnight",
        "--date=local",
    ];

    if !args.all {
        log_args.push("--author");
        // Use the configured git user
        log_args.push("$(git config user.email)");
    }

    // Use passthrough for colors
    if args.all {
        git::run(&["log", "--oneline", "--since=midnight", "--date=local"])
    } else {
        // Get current user email first
        match git::capture(&["config", "user.email"]) {
            Ok(email) => {
                let author_arg = format!("--author={}", email);
                git::run(&["log", "--oneline", "--since=midnight", "--date=local", &author_arg])
            }
            Err(_) => {
                // Fall back to showing all
                git::run(&["log", "--oneline", "--since=midnight", "--date=local"])
            }
        }
    }
}
