use clap::Args;
use colored::Colorize;

use crate::git;
use crate::utils::{get_branch_name, get_repo, is_main_branch};

#[derive(Args)]
pub struct PushArgs {
    /// Force push (use with caution)
    #[arg(short, long)]
    pub force: bool,
}

pub fn run(args: PushArgs) -> i32 {
    match run_inner(args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("gg: {}", e);
            1
        }
    }
}

fn run_inner(args: PushArgs) -> Result<i32, Box<dyn std::error::Error>> {
    let repo = get_repo()?;
    let branch_name = get_branch_name(&repo).ok_or("Could not determine current branch")?;

    let mut push_args: Vec<&str> = vec!["push"];

    if args.force {
        push_args.push("--force-with-lease");
    }

    // Auto-set upstream for non-main branches
    if !is_main_branch(&branch_name) {
        // Check if upstream is already set
        let has_upstream = repo
            .find_branch(&branch_name, git2::BranchType::Local)
            .ok()
            .and_then(|b| b.upstream().ok())
            .is_some();

        if !has_upstream {
            push_args.extend(["--set-upstream", "origin"]);
            push_args.push(Box::leak(branch_name.clone().into_boxed_str()));
        }
    }

    println!("Running: {}", format!("git {}", push_args.join(" ")).bold());
    println!();

    Ok(git::run(&push_args))
}
