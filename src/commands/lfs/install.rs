//! Install/uninstall git hooks for LFS

use crate::lfs::LfsConfig;
use clap::Args;
use colored::Colorize;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

#[derive(Args, Debug)]
pub struct InstallArgs {
    /// Force overwrite existing hooks
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct UninstallArgs {}

/// Hook script content
const PRE_PUSH_HOOK: &str = r#"#!/bin/sh
# gg-lfs pre-push hook
# Automatically push LFS files before git push

exec gg lfs push
"#;

const POST_CHECKOUT_HOOK: &str = r#"#!/bin/sh
# gg-lfs post-checkout hook
# Automatically pull LFS files after checkout

exec gg lfs pull
"#;

const POST_MERGE_HOOK: &str = r#"#!/bin/sh
# gg-lfs post-merge hook
# Automatically pull LFS files after merge

exec gg lfs pull
"#;

/// Install LFS hooks
pub fn run(args: InstallArgs) -> i32 {
    match run_inner(args) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            1
        }
    }
}

fn run_inner(args: InstallArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Find repository root
    let repo = git2::Repository::discover(".")?;
    let repo_root = repo
        .workdir()
        .ok_or("Not a git repository with a working directory")?;

    let hooks_dir = repo_root.join(".git").join("hooks");
    fs::create_dir_all(&hooks_dir)?;

    // Install hooks
    let hooks = [
        ("pre-push", PRE_PUSH_HOOK),
        ("post-checkout", POST_CHECKOUT_HOOK),
        ("post-merge", POST_MERGE_HOOK),
    ];

    for (name, content) in hooks {
        let hook_path = hooks_dir.join(name);

        if hook_path.exists() && !args.force {
            // Check if it's our hook
            let existing = fs::read_to_string(&hook_path)?;
            if !existing.contains("gg-lfs") {
                println!(
                    "{} {} exists (use -f to overwrite)",
                    "Skipping:".yellow(),
                    name
                );
                continue;
            }
        }

        fs::write(&hook_path, content)?;

        // Make executable
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;

        println!("{} {}", "Installed:".green(), name);
    }

    // Create config template if it doesn't exist
    if !LfsConfig::exists(repo_root) {
        let config_path = LfsConfig::write_template(repo_root)?;
        println!(
            "{} {}",
            "Created:".green(),
            config_path.strip_prefix(repo_root).unwrap_or(&config_path).display()
        );
        println!(
            "{}",
            "Edit .gg/lfs.toml to configure your storage backend".cyan()
        );
    }

    // Add .gg/ to .gitignore if not already there
    add_to_gitignore(repo_root)?;

    println!("{}", "LFS hooks installed successfully!".green().bold());
    Ok(())
}

/// Uninstall LFS hooks
pub fn run_uninstall(_args: UninstallArgs) -> i32 {
    match run_uninstall_inner() {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            1
        }
    }
}

fn run_uninstall_inner() -> Result<(), Box<dyn std::error::Error>> {
    let repo = git2::Repository::discover(".")?;
    let repo_root = repo
        .workdir()
        .ok_or("Not a git repository with a working directory")?;

    let hooks_dir = repo_root.join(".git").join("hooks");

    let hooks = ["pre-push", "post-checkout", "post-merge"];

    for name in hooks {
        let hook_path = hooks_dir.join(name);

        if hook_path.exists() {
            let content = fs::read_to_string(&hook_path)?;
            if content.contains("gg-lfs") {
                fs::remove_file(&hook_path)?;
                println!("{} {}", "Removed:".green(), name);
            } else {
                println!(
                    "{} {} (not a gg-lfs hook)",
                    "Skipping:".yellow(),
                    name
                );
            }
        }
    }

    println!("{}", "LFS hooks uninstalled.".green().bold());
    Ok(())
}

/// Add .gg/ to .gitignore
fn add_to_gitignore(repo_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let gitignore = repo_root.join(".gitignore");

    let content = if gitignore.exists() {
        fs::read_to_string(&gitignore)?
    } else {
        String::new()
    };

    // Check if .gg/ is already ignored
    for line in content.lines() {
        let line = line.trim();
        if line == ".gg/" || line == ".gg" || line == "/.gg/" || line == "/.gg" {
            return Ok(());
        }
    }

    // Add to .gitignore
    let new_content = if content.ends_with('\n') || content.is_empty() {
        format!("{}# gg-lfs config (contains credentials)\n.gg/\n", content)
    } else {
        format!(
            "{}\n\n# gg-lfs config (contains credentials)\n.gg/\n",
            content
        )
    };

    fs::write(&gitignore, new_content)?;
    println!("{} .gg/ to .gitignore", "Added:".green());

    Ok(())
}
