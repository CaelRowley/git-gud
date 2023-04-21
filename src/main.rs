use std::{env, path::PathBuf, process::Command};

use colored::*;
use git2::{Repository, StatusOptions};


fn main() {
    let cli_args: Vec<String> = env::args().collect();

    let command = cli_args[1].clone();

    let mut args = Vec::new();
    for arg in cli_args.iter().skip(1) {
        args.push(arg.clone());
    }

    match command.as_str() {
        "clone" | "c" => clone(args),
        "status" | "s" => status(args),
        _ => default(args),
    }
}


fn clone(args: Vec<String>) {
    let url = args[1].to_owned();
    let repo = match Repository::clone(&url, "./test") {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to clone repo: {}", e),
    };
    println!("Repo cloned to: {}\n", repo.path().parent().unwrap().to_string_lossy().bold());
}


fn status(_args: Vec<String>) {
    let repo = match Repository::open(".") {
        Ok(repo) => repo,
        Err(e) => panic!("No repo in current dir: {}", e),
    };

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);

    let statuses = match repo.statuses(Some(&mut opts)) {
        Ok(statuses) => statuses,
        Err(e) => panic!("oops {}", e),
    };
    println!("On branch: {}\n", repo.head().unwrap().name().unwrap().to_string().bold());

    let mut staged = vec![];
    let mut unstaged = vec![];
    let mut untracked = vec![];
    let mut deleted = vec![];
    let mut unkown = vec![];
    
    for entry in statuses.iter() {
        let path = entry.path().unwrap().to_owned();

        let status = entry.status();
        if status.is_index_new() || status.is_index_modified() {
            staged.push(path.clone());
        } 
        if status.is_wt_modified() {
            unstaged.push(path);
        } else if status.is_wt_new() {
            untracked.push(path);
        } else if status.is_wt_deleted() {
            deleted.push(path);
        }
        else {
            if !staged.contains(&path) {
                unkown.push(path);
            }
        }
    }

    if !staged.is_empty() {
        println!("{}", "Changes to be committed:".bold().green());
        for path in staged {
            let path_buf = PathBuf::from(path);
            println!("{}", format!("  new file: {}", path_buf.display()).green());
        }
        println!();
    }
    if !unstaged.is_empty() {
        println!("{}", "Changes not staged for commit:".bold().yellow());
        for path in unstaged {
            let path_buf = PathBuf::from(path);
            println!("{}", format!("  modified: {}", path_buf.display()).yellow());
        }
        println!();
    }
    if !untracked.is_empty() {
        println!("{}", "Untracked files:".bold().red());
        for path in untracked {
            let path_buf = PathBuf::from(path);
            println!("{}", format!("  {}", path_buf.display()).red());
        }
        println!();
    }
    if !deleted.is_empty() {
        println!("{}", "Deleted files:".bold().red());
        for path in deleted {
            let path_buf = PathBuf::from(path);
            println!("{}", format!("  {}", path_buf.display()).red());
        }
        println!();
    }
    if !unkown.is_empty() {
        println!("{}", "Unkown files:".bold());
        for path in unkown {
            let path_buf = PathBuf::from(path);
            println!("{}", format!(" {}", path_buf.display()))
        }
        println!();
    }
}


fn default(args: Vec<String>) {
    let default_command = "git";

    let command_str = format!("{} {}", default_command, args.join(" "));
    println!("Running default command: {}", command_str.bold());

    let output = Command::new(default_command)
        .args(args)
        .output()
        .expect(&format!("Failed to execute command '{}'", default_command));

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout);
        if !result.is_empty() {
            println!("Command output: {}", result);
        }
    } else {
        let result = String::from_utf8_lossy(&output.stderr);
        if !result.is_empty() {
            println!("Command error: {}", result);
        }
    }
}