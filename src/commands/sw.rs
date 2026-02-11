use clap::Args;
use colored::Colorize;
use std::io::{self, Write};

use crate::git;

#[derive(Args)]
pub struct SwArgs {
    /// Branch number to switch to (from gg recent)
    pub number: Option<usize>,
}

pub fn run(args: SwArgs) -> i32 {
    match run_inner(args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("gg: {}", e);
            1
        }
    }
}

fn run_inner(args: SwArgs) -> Result<i32, Box<dyn std::error::Error>> {
    let branches = get_recent_branches(10)?;

    if branches.is_empty() {
        println!("No recent branches found.");
        return Ok(0);
    }

    let selected = match args.number {
        Some(n) if n > 0 && n <= branches.len() => n - 1,
        Some(n) => {
            eprintln!("Invalid selection: {}. Choose 1-{}", n, branches.len());
            return Ok(1);
        }
        None => {
            // Interactive mode: show list and prompt
            println!("{}", "Recent branches:".bold());
            for (i, branch) in branches.iter().enumerate() {
                let num = format!("{:>2}", i + 1);
                println!("  {} {}", num.dimmed(), branch.cyan());
            }
            println!();
            print!("Switch to (1-{}): ", branches.len());
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let n: usize = input.trim().parse().map_err(|_| "Invalid number")?;

            if n == 0 || n > branches.len() {
                eprintln!("Invalid selection");
                return Ok(1);
            }
            n - 1
        }
    };

    let branch = &branches[selected];
    println!("Switching to: {}", branch.cyan());
    Ok(git::run(&["checkout", branch]))
}

fn get_recent_branches(count: usize) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let reflog = git::capture(&[
        "reflog",
        "show",
        "--pretty=format:%gs",
        "-n",
        "100",
    ])?;

    let mut seen = std::collections::HashSet::new();
    let mut branches = Vec::new();

    for line in reflog.lines() {
        if let Some(rest) = line.strip_prefix("checkout: moving from ") {
            if let Some(to_idx) = rest.find(" to ") {
                let to_branch = &rest[to_idx + 4..];
                if !to_branch.contains(' ') && !to_branch.starts_with("HEAD") {
                    if seen.insert(to_branch.to_string()) {
                        branches.push(to_branch.to_string());
                        if branches.len() >= count {
                            break;
                        }
                    }
                }
            }
        }
    }

    Ok(branches)
}
