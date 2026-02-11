use clap::Args;
use colored::Colorize;

use crate::git;
use crate::utils::{get_branch_name, get_main_branch_name, get_repo, is_main_branch};

#[derive(Args)]
pub struct SyncArgs {
    /// Don't stash changes before syncing
    #[arg(long)]
    pub no_stash: bool,
}

pub fn run(args: SyncArgs) -> i32 {
    match run_inner(args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("gg: {}", e);
            1
        }
    }
}

fn run_inner(args: SyncArgs) -> Result<i32, Box<dyn std::error::Error>> {
    let repo = get_repo()?;
    let branch_name = get_branch_name(&repo).ok_or("Could not determine current branch")?;

    if is_main_branch(&branch_name) {
        sync_on_main()
    } else {
        sync_on_branch(&branch_name, &repo, args.no_stash)
    }
}

fn sync_on_main() -> Result<i32, Box<dyn std::error::Error>> {
    println!("Running: {}", "git pull --rebase".bold());
    println!();
    Ok(git::run(&["pull", "--rebase"]))
}

fn sync_on_branch(
    branch_name: &str,
    repo: &git2::Repository,
    no_stash: bool,
) -> Result<i32, Box<dyn std::error::Error>> {
    let main_branch = get_main_branch_name(repo);

    // Build command sequence
    let stash_cmd: &[&str] = &["stash"];
    let checkout_main: Vec<&str> = vec!["checkout", main_branch];
    let pull_rebase: &[&str] = &["pull", "--rebase"];
    let checkout_branch: Vec<&str> = vec!["checkout", branch_name];
    let rebase_main: Vec<&str> = vec!["rebase", main_branch];
    let stash_pop: &[&str] = &["stash", "pop"];

    let mut commands: Vec<&[&str]> = vec![];

    if !no_stash {
        commands.push(stash_cmd);
    }
    commands.push(&checkout_main);
    commands.push(pull_rebase);
    commands.push(&checkout_branch);
    commands.push(&rebase_main);
    if !no_stash {
        commands.push(stash_pop);
    }

    for cmd in &commands {
        println!("Running: {}", format!("git {}", cmd.join(" ")).bold());
        let code = git::run(cmd);
        if code != 0 {
            return Ok(code);
        }
        println!();
    }

    Ok(0)
}
