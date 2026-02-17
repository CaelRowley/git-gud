//! Clean filter for LFS
//!
//! Invoked by git during `git add` via the filter driver:
//!   filter.lfs.clean = "gg lfs clean %f"
//!
//! Reads file content from stdin, outputs pointer text to stdout.
//! Caches the original content locally (no network access).

use crate::lfs::{Cache, Pointer};
use clap::Args;
use std::io::{self, Read, Write};

#[derive(Args, Debug)]
pub struct CleanArgs {
    /// The file path (passed by git as %f, used for diagnostics only)
    pub file: Option<String>,
}

/// Run the clean filter
pub fn run(args: CleanArgs) -> i32 {
    match run_inner(args) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("gg lfs clean: {}", e);
            1
        }
    }
}

fn run_inner(_args: CleanArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Read all content from stdin
    let mut content = Vec::new();
    io::stdin().read_to_end(&mut content)?;

    // If content is already a pointer, pass through unchanged
    if content.len() <= 1024 {
        if let Ok(pointer) = Pointer::parse_content(io::BufReader::new(content.as_slice())) {
            // Valid pointer — pass through as-is
            io::stdout().write_all(&content)?;
            io::stdout().flush()?;
            // Still cache it if we can parse the oid, in case the real content
            // was previously cached by someone else
            let _ = pointer; // already validated
            return Ok(());
        }
    }

    // Hash content and create pointer
    let pointer = Pointer::from_bytes(&content);
    let oid = pointer.sha256().to_string();

    // Cache locally (best-effort — don't fail the filter if cache fails)
    if let Ok(cache) = Cache::new() {
        let _ = cache.put(&oid, &content);
    }

    // Write pointer text to stdout
    let pointer_text = format!("{}\n", pointer);
    io::stdout().write_all(pointer_text.as_bytes())?;
    io::stdout().flush()?;

    Ok(())
}
