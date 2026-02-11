use clap::Args;
use colored::Colorize;

use crate::git;

#[derive(Args)]
pub struct RecentArgs {
    /// Number of recent branches to show (default: 10)
    #[arg(short, long, default_value = "10")]
    pub count: usize,
}

pub fn run(args: RecentArgs) -> i32 {
    match run_inner(args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("gg: {}", e);
            1
        }
    }
}

fn run_inner(args: RecentArgs) -> Result<i32, Box<dyn std::error::Error>> {
    // Get reflog entries for checkout operations
    let reflog = git::capture(&[
        "reflog",
        "show",
        "--pretty=format:%gs",
        "--date=relative",
        "-n",
        "100",
    ])?;

    let mut seen = std::collections::HashSet::new();
    let mut branches = Vec::new();

    for line in reflog.lines() {
        // Parse "checkout: moving from X to Y"
        if let Some(rest) = line.strip_prefix("checkout: moving from ") {
            if let Some(to_idx) = rest.find(" to ") {
                let to_branch = &rest[to_idx + 4..];
                // Skip detached HEAD states
                if !to_branch.contains(' ') && !to_branch.starts_with("HEAD") {
                    if seen.insert(to_branch.to_string()) {
                        branches.push(to_branch.to_string());
                        if branches.len() >= args.count {
                            break;
                        }
                    }
                }
            }
        }
    }

    if branches.is_empty() {
        println!("No recent branches found.");
        return Ok(0);
    }

    println!("{}", "Recent branches:".bold());
    for (i, branch) in branches.iter().enumerate() {
        let num = format!("{:>2}", i + 1);
        println!("  {} {}", num.dimmed(), branch.cyan());
    }

    Ok(0)
}
