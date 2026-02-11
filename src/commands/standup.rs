use clap::Args;

use crate::git;

#[derive(Args)]
pub struct StandupArgs {
    /// Show all authors, not just yours
    #[arg(short, long)]
    pub all: bool,

    /// Number of days to look back (default: auto-detect last workday)
    #[arg(short, long)]
    pub days: Option<u32>,
}

pub fn run(args: StandupArgs) -> i32 {
    // Calculate since date
    let since = match args.days {
        Some(d) => format!("{} days ago", d),
        None => calculate_last_workday(),
    };

    if args.all {
        git::run(&[
            "log",
            "--oneline",
            "--since",
            &since,
            "--date=local",
        ])
    } else {
        match git::capture(&["config", "user.email"]) {
            Ok(email) => {
                let author_arg = format!("--author={}", email);
                git::run(&[
                    "log",
                    "--oneline",
                    "--since",
                    &since,
                    "--date=local",
                    &author_arg,
                ])
            }
            Err(_) => git::run(&[
                "log",
                "--oneline",
                "--since",
                &since,
                "--date=local",
            ]),
        }
    }
}

fn calculate_last_workday() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Get current day of week (0 = Sunday, 6 = Saturday)
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Days since Unix epoch
    let days_since_epoch = now / 86400;
    // Day of week (0 = Thursday because Unix epoch was Thursday)
    let day_of_week = (days_since_epoch + 4) % 7; // 0 = Sunday

    let days_back = match day_of_week {
        0 => 2, // Sunday -> Friday
        1 => 3, // Monday -> Friday
        _ => 1, // Other days -> yesterday
    };

    format!("{} days ago midnight", days_back)
}
