//! Install/uninstall git hooks for LFS

use crate::lfs::LfsConfig;
use clap::Args;
use colored::Colorize;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

#[derive(Args, Debug)]
pub struct InstallArgs {
    /// Force overwrite existing hooks
    #[arg(short, long)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct UninstallArgs {}

/// Generate hook script content using the full path to the gg binary
fn pre_push_hook(gg_path: &str) -> String {
    format!(
        "#!/bin/sh\n# gg-lfs pre-push hook\n# Automatically push LFS files before git push\n\nexec {} lfs push --pre-push\n",
        gg_path
    )
}

fn post_checkout_hook(gg_path: &str) -> String {
    format!(
        "#!/bin/sh\n# gg-lfs post-checkout hook\n# Automatically pull LFS files after checkout\n\nexec {} lfs pull --post-checkout \"$1\" \"$2\" \"$3\"\n",
        gg_path
    )
}

fn post_merge_hook(gg_path: &str) -> String {
    format!(
        "#!/bin/sh\n# gg-lfs post-merge hook\n# Automatically pull LFS files after merge\n\nexec {} lfs pull --post-merge\n",
        gg_path
    )
}

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

    // Resolve gg binary path for hooks
    let gg_bin = std::env::current_exe()?;
    let gg_path = gg_bin.to_string_lossy().to_string();

    // Install hooks
    let hooks = [
        ("pre-push", pre_push_hook(&gg_path)),
        ("post-checkout", post_checkout_hook(&gg_path)),
        ("post-merge", post_merge_hook(&gg_path)),
    ];

    for (name, content) in &hooks {
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
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&hook_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&hook_path, perms)?;
        }

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

    // Migrate old filter name if needed
    migrate_filter_name(repo_root)?;

    // Register filter driver in git config
    register_filter_driver(repo_root)?;

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

    // Remove filter driver from git config
    unregister_filter_driver(repo_root);

    println!("{}", "LFS hooks uninstalled.".green().bold());
    Ok(())
}

/// Register the gg lfs filter driver in git config
pub fn register_filter_driver(repo_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Use the full path to the current binary so the filter works even if
    // `gg` is not yet in PATH (e.g. running from cargo build directory).
    let gg_bin = std::env::current_exe()?;
    let gg_path = gg_bin.to_string_lossy();

    let clean_cmd = format!("{} lfs clean %f", gg_path);
    let smudge_cmd = format!("{} lfs smudge %f", gg_path);
    let process_cmd = format!("{} lfs filter-process", gg_path);

    let configs = [
        ("filter.gg-lfs.clean", clean_cmd.as_str()),
        ("filter.gg-lfs.smudge", smudge_cmd.as_str()),
        ("filter.gg-lfs.process", process_cmd.as_str()),
        ("filter.gg-lfs.required", "true"),
    ];

    for (key, value) in configs {
        let status = Command::new("git")
            .args(["config", key, value])
            .current_dir(repo_root)
            .status()?;
        if !status.success() {
            return Err(format!("Failed to set {}", key).into());
        }
    }

    println!("{} filter driver (clean/smudge/process)", "Registered:".green());
    Ok(())
}

/// Remove the gg lfs filter driver from git config
pub fn unregister_filter_driver(repo_root: &Path) {
    // Always remove our gg-lfs keys
    for key in ["filter.gg-lfs.clean", "filter.gg-lfs.smudge", "filter.gg-lfs.required", "filter.gg-lfs.process"] {
        let _ = Command::new("git")
            .args(["config", "--unset", key])
            .current_dir(repo_root)
            .status();
    }

    // Only remove old filter.lfs keys if they point to our command
    for key in ["filter.lfs.clean", "filter.lfs.smudge", "filter.lfs.required"] {
        let output = Command::new("git")
            .args(["config", key])
            .current_dir(repo_root)
            .output();
        if let Ok(output) = output {
            let value = String::from_utf8_lossy(&output.stdout);
            if value.contains("gg lfs") {
                let _ = Command::new("git")
                    .args(["config", "--unset", key])
                    .current_dir(repo_root)
                    .status();
            }
        }
    }

    println!("{} filter driver", "Removed:".green());
}

/// Migrate old `filter=lfs` entries to `filter=gg-lfs` in .gitattributes
/// and unregister the old filter.lfs.* git config keys.
/// Only migrates if the old filter.lfs.clean was set to our command ("gg lfs").
fn migrate_filter_name(repo_root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let gitattributes = repo_root.join(".gitattributes");
    if !gitattributes.exists() {
        return Ok(());
    }

    // Check if old filter.lfs.clean points to our command
    let output = Command::new("git")
        .args(["config", "filter.lfs.clean"])
        .current_dir(repo_root)
        .output()?;
    let old_clean = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !old_clean.contains("gg lfs") {
        return Ok(());
    }

    // Check if .gitattributes has old-style filter=lfs entries
    let content = fs::read_to_string(&gitattributes)?;
    if !content.contains("filter=lfs") {
        return Ok(());
    }

    // Replace filter=lfs with filter=gg-lfs (and diff=, merge=)
    let new_content = content
        .replace("filter=lfs", "filter=gg-lfs")
        .replace("diff=lfs", "diff=gg-lfs")
        .replace("merge=lfs", "merge=gg-lfs");
    fs::write(&gitattributes, new_content)?;

    // Unregister old filter.lfs.* keys
    for key in ["filter.lfs.clean", "filter.lfs.smudge", "filter.lfs.required"] {
        let _ = Command::new("git")
            .args(["config", "--unset", key])
            .current_dir(repo_root)
            .status();
    }

    println!("{} .gitattributes filter=lfs -> filter=gg-lfs", "Migrated:".green());
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
