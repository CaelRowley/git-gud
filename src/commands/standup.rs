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
    let days_back = days_since_last_workday();
    format!("{} days ago midnight", days_back)
}

/// Calculate the number of days since the last workday.
/// Returns 1 for Tue-Sat (yesterday), 2 for Sunday (Friday), 3 for Monday (Friday).
fn days_since_last_workday() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Days since Unix epoch
    let days_since_epoch = now / 86400;
    // Day of week: 0 = Sunday, 1 = Monday, ..., 6 = Saturday
    // Unix epoch (Jan 1, 1970) was a Thursday, so we add 4
    let day_of_week = (days_since_epoch + 4) % 7;

    match day_of_week {
        0 => 2, // Sunday -> Friday (2 days back)
        1 => 3, // Monday -> Friday (3 days back)
        _ => 1, // Tue-Sat -> yesterday
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_days_since_last_workday_returns_valid_range() {
        let days = days_since_last_workday();
        // Should always be 1, 2, or 3
        assert!(days >= 1 && days <= 3, "days_back was {}", days);
    }

    #[test]
    fn test_calculate_last_workday_format() {
        let result = calculate_last_workday();
        assert!(result.ends_with(" days ago midnight"));
        assert!(
            result.starts_with("1 ") || result.starts_with("2 ") || result.starts_with("3 "),
            "Unexpected format: {}",
            result
        );
    }
}
