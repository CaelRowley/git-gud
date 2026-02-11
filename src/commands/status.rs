use std::path::PathBuf;

use clap::Args;
use colored::Colorize;
use git2::StatusOptions;

use crate::config::Theme;
use crate::utils::get_repo;

#[derive(Args)]
pub struct StatusArgs {
    /// Show short format
    #[arg(short, long)]
    pub short: bool,
}

pub fn run(args: StatusArgs) -> i32 {
    if args.short {
        // Delegate to git for short format
        return crate::git::run(&["status", "-s"]);
    }

    match run_inner() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("gg: {}", e);
            1
        }
    }
}

fn run_inner() -> Result<(), Box<dyn std::error::Error>> {
    let repo = get_repo()?;
    let theme = Theme::default();

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);

    let statuses = repo.statuses(Some(&mut opts))?;

    // Print branch info
    let head = repo.head()?;
    let branch_name = head.shorthand().unwrap_or("HEAD");
    println!(
        "On branch: {}\n",
        branch_name.color(theme.branch).bold()
    );

    let mut staged = vec![];
    let mut unstaged = vec![];
    let mut untracked = vec![];
    let mut deleted = vec![];

    for entry in statuses.iter() {
        let path = entry.path().unwrap_or("").to_owned();
        let status = entry.status();

        if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() {
            staged.push((path.clone(), status));
        }

        if status.is_wt_modified() {
            unstaged.push(path);
        } else if status.is_wt_new() {
            untracked.push(path);
        } else if status.is_wt_deleted() {
            deleted.push(path);
        }
    }

    let has_changes =
        !staged.is_empty() || !unstaged.is_empty() || !untracked.is_empty() || !deleted.is_empty();

    if !staged.is_empty() {
        println!("{}", "Changes to be committed:".bold().color(theme.staged));
        for (path, status) in &staged {
            let path_buf = PathBuf::from(path);
            let prefix = if status.is_index_new() {
                "new file:"
            } else if status.is_index_deleted() {
                "deleted:"
            } else {
                "modified:"
            };
            println!(
                "{}",
                format!("  {} {}", prefix, path_buf.display()).color(theme.staged)
            );
        }
        println!();
    }

    if !unstaged.is_empty() {
        println!(
            "{}",
            "Changes not staged for commit:".bold().color(theme.modified)
        );
        for path in &unstaged {
            let path_buf = PathBuf::from(path);
            println!(
                "{}",
                format!("  modified: {}", path_buf.display()).color(theme.modified)
            );
        }
        println!();
    }

    if !untracked.is_empty() {
        println!("{}", "Untracked files:".bold().color(theme.untracked));
        for path in &untracked {
            let path_buf = PathBuf::from(path);
            println!(
                "{}",
                format!("  {}", path_buf.display()).color(theme.untracked)
            );
        }
        println!();
    }

    if !deleted.is_empty() {
        println!("{}", "Deleted files:".bold().color(theme.deleted));
        for path in &deleted {
            let path_buf = PathBuf::from(path);
            println!(
                "{}",
                format!("  {}", path_buf.display()).color(theme.deleted)
            );
        }
        println!();
    }

    if !has_changes {
        println!("nothing to commit, working tree clean");
    }

    Ok(())
}
