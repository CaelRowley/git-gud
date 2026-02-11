use clap::Args;
use colored::Colorize;

use crate::git;
use crate::utils::{get_main_branch_name, get_repo};

#[derive(Args)]
pub struct CleanBranchesArgs {
    /// Actually delete branches (dry-run by default)
    #[arg(short, long)]
    pub force: bool,
}

pub fn run(args: CleanBranchesArgs) -> i32 {
    match run_inner(args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("gg: {}", e);
            1
        }
    }
}

fn run_inner(args: CleanBranchesArgs) -> Result<i32, Box<dyn std::error::Error>> {
    let repo = get_repo()?;
    let main_branch = get_main_branch_name(&repo);

    // Get list of merged branches
    let merged_output = git::capture(&["branch", "--merged", main_branch])?;

    let branches_to_delete: Vec<&str> = merged_output
        .lines()
        .map(|line| line.trim().trim_start_matches("* "))
        .filter(|branch| !branch.is_empty())
        .filter(|branch| *branch != "main" && *branch != "master" && *branch != main_branch)
        .collect();

    if branches_to_delete.is_empty() {
        println!("No merged branches to clean up.");
        return Ok(0);
    }

    if !args.force {
        println!("{}", "Branches that would be deleted (dry-run):".bold());
        for branch in &branches_to_delete {
            println!("  {}", branch.red());
        }
        println!();
        println!("Run with {} to actually delete these branches.", "--force".bold());
        return Ok(0);
    }

    println!("{}", "Deleting merged branches:".bold());
    for branch in branches_to_delete {
        println!("  Deleting: {}", branch.red());
        let code = git::run(&["branch", "-d", branch]);
        if code != 0 {
            eprintln!("  Failed to delete {}", branch);
        }
    }

    Ok(0)
}
