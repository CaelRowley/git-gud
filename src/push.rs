use std::process::Command;

use git2::{Repository};
use colored::*;


pub fn push(_args: Vec<String>) {
    let repo = match Repository::open(".") {
        Ok(repo) => repo,
        Err(e) => panic!("No repo in current dir: {}", e),
    };
    let head = repo.head().unwrap();
    let branch_name = head.shorthand().unwrap();

    let mut push_args = vec![];
    match branch_name {
        "main" | "master" => {
            push_args.push("push")
        }
        _ => {
            push_args.extend(["push", "--set-upstream", "origin", branch_name])
        }
    }

    let default_command = "git";

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