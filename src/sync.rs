use std::{process::Command};

use git2::{Repository, BranchType};
use colored::*;


pub fn sync(_args: Vec<String>) {
    let repo = match Repository::open(".") {
        Ok(repo) => repo,
        Err(e) => panic!("No repo in current dir: {}", e),
    };
    let head = repo.head().unwrap();
    let branch_name = head.shorthand().unwrap();

    match branch_name {
        "main" | "master" => sync_on_master(),
        _ => sync_on_branch(branch_name, &repo),
    }
}


fn sync_on_master() {
    let default_command = "git";
    let push_args = vec!["pull", "--rebase"];

    let command_str = format!("{} {}", default_command, push_args.join(" "));
    println!("Running: {}", command_str.bold());
    println!();

    let output = Command::new(default_command)
        .args(push_args)
        .output()
        .expect(&format!("Failed to execute command '{}'", default_command));

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout);
        if !result.is_empty() {
            println!("Output: {}", result.bold());
        }
    } else {
        let result = String::from_utf8_lossy(&output.stderr);
        if !result.is_empty() {
            println!("{}", format!("Error: {}", result.bold()).red());
        }
    }
}


fn sync_on_branch(branch_name: &str, repo: &Repository) {
    let main_branch = match repo.find_branch("main", BranchType::Local) {
        Ok(main_branch) => main_branch,
        Err(e) => panic!("No repo in current dir: {}", e),
    };

    let main_branch_name = if main_branch.name().unwrap() == Some("main") { "main" } else { "master" };

    run_command(["stash"].to_vec());
    run_command(["checkout", main_branch_name].to_vec());
    run_command(["pull", "--rebase"].to_vec());
    run_command(["checkout", branch_name].to_vec());
    run_command(["rebase", main_branch_name].to_vec());
}


fn run_command(command_args: Vec<&str>) {
    let default_command = "git";
    let command_str = format!("{} {}", default_command, command_args.join(" "));
    println!("Running: {}", command_str.bold());
    println!();

    let output = Command::new(default_command)
        .args(command_args)
        .output()
        .expect(&format!("Failed to execute command '{}'", default_command));

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout);
        if !result.is_empty() {
            println!("Output: {}", result.bold());
        }
    } else {
        let result = String::from_utf8_lossy(&output.stderr);
        if !result.is_empty() {
            println!("{}", format!("Error: {}", result.bold()).red());
        }
    }
}
