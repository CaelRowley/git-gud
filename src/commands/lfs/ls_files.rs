//! List LFS-tracked files

use crate::lfs::{Pointer, Scanner};
use clap::Args;
use colored::Colorize;

#[derive(Args, Debug)]
pub struct LsFilesArgs {
    /// Show OID and size for each file
    #[arg(short, long)]
    pub long: bool,
}

/// List LFS-tracked files
pub fn run(args: LsFilesArgs) -> i32 {
    match run_inner(args) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            1
        }
    }
}

fn run_inner(args: LsFilesArgs) -> Result<(), Box<dyn std::error::Error>> {
    let repo = git2::Repository::discover(".")?;
    let repo_root = repo
        .workdir()
        .ok_or("Not a git repository with a working directory")?;

    let scanner = Scanner::new(repo_root)?;

    if scanner.patterns().is_empty() {
        println!("{}", "No LFS patterns defined.".dimmed());
        return Ok(());
    }

    let files = scanner.scan_files()?;

    if files.is_empty() {
        println!("{}", "No LFS files found.".dimmed());
        return Ok(());
    }

    for file_path in &files {
        let relative = file_path.strip_prefix(repo_root).unwrap_or(file_path);

        if args.long {
            let (oid_short, size, kind) = if Pointer::is_pointer_file(file_path) {
                match Pointer::parse(file_path) {
                    Ok(p) => {
                        let oid = p.sha256();
                        let short = if oid.len() > 12 { &oid[..12] } else { oid };
                        (short.to_string(), p.size, "pointer")
                    }
                    Err(_) => ("???".to_string(), 0, "pointer"),
                }
            } else {
                match Pointer::from_file(file_path) {
                    Ok(p) => {
                        let oid = p.sha256();
                        let short = if oid.len() > 12 { &oid[..12] } else { oid };
                        (short.to_string(), p.size, "real")
                    }
                    Err(_) => ("???".to_string(), 0, "real"),
                }
            };
            println!(
                "{} {:>10}  {} ({})",
                oid_short.dimmed(),
                format_bytes(size),
                relative.display(),
                kind
            );
        } else {
            println!("{}", relative.display());
        }
    }

    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
