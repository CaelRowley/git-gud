//! Smudge filter for LFS
//!
//! Invoked by git during `git checkout` via the filter driver:
//!   filter.lfs.smudge = "gg lfs smudge %f"
//!
//! Reads pointer text from stdin, outputs real file content to stdout.
//! Checks local cache first, falls back to S3 download on cache miss.

use crate::lfs::storage::{S3Config, S3Storage, Storage};
use crate::lfs::{Cache, LfsConfig, Pointer};
use clap::Args;
use std::io::{self, BufReader, Read, Write};

#[derive(Args, Debug)]
pub struct SmudgeArgs {
    /// The file path (passed by git as %f, used for diagnostics only)
    pub file: Option<String>,
}

/// Run the smudge filter
pub fn run(args: SmudgeArgs) -> i32 {
    match run_inner(args) {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("gg lfs smudge: {}", e);
            1
        }
    }
}

fn run_inner(args: SmudgeArgs) -> Result<(), Box<dyn std::error::Error>> {
    // Read all content from stdin
    let mut content = Vec::new();
    io::stdin().read_to_end(&mut content)?;

    // Try to parse as pointer — if not a pointer, pass through unchanged
    let pointer = match Pointer::parse_content(BufReader::new(content.as_slice())) {
        Ok(p) => p,
        Err(_) => {
            // Not a pointer, pass through as-is
            io::stdout().write_all(&content)?;
            io::stdout().flush()?;
            return Ok(());
        }
    };

    let oid = pointer.sha256().to_string();

    // Check local cache first
    if let Ok(cache) = Cache::new() {
        if let Ok(data) = cache.read(&oid) {
            io::stdout().write_all(&data)?;
            io::stdout().flush()?;
            return Ok(());
        }
    }

    // Cache miss — try S3 download
    let file_hint = args.file.as_deref().unwrap_or("<unknown>");

    let repo = match git2::Repository::discover(".") {
        Ok(r) => r,
        Err(_) => {
            // Can't find repo, output pointer as-is (graceful degradation)
            eprintln!(
                "gg lfs smudge: warning: cannot find repository for {}, outputting pointer",
                file_hint
            );
            io::stdout().write_all(&content)?;
            io::stdout().flush()?;
            return Ok(());
        }
    };

    let repo_root = match repo.workdir() {
        Some(r) => r,
        None => {
            eprintln!(
                "gg lfs smudge: warning: bare repository, outputting pointer for {}",
                file_hint
            );
            io::stdout().write_all(&content)?;
            io::stdout().flush()?;
            return Ok(());
        }
    };

    let config = match LfsConfig::load(repo_root) {
        Ok(c) => c,
        Err(_) => {
            eprintln!(
                "gg lfs smudge: warning: no LFS config, outputting pointer for {}",
                file_hint
            );
            io::stdout().write_all(&content)?;
            io::stdout().flush()?;
            return Ok(());
        }
    };

    // Need async runtime for S3 download
    let rt = tokio::runtime::Runtime::new()?;
    let result = rt.block_on(async {
        let s3_config = S3Config {
            bucket: config.storage.bucket.clone(),
            region: config.storage.region.clone(),
            prefix: config.storage.prefix.clone(),
            endpoint: config.storage.endpoint.clone(),
            credentials: config.storage.credentials.as_ref().map(|c| {
                crate::lfs::storage::s3::S3Credentials {
                    access_key_id: c.access_key_id.clone(),
                    secret_access_key: c.secret_access_key.clone(),
                }
            }),
        };

        let storage = S3Storage::new(s3_config).await?;

        // Download to a temp file
        let temp_dir = repo_root.join(".gg").join("tmp");
        std::fs::create_dir_all(&temp_dir)?;
        let temp_path = temp_dir.join(&oid);

        storage.download(&oid, &temp_path).await?;

        // Verify hash
        let downloaded_pointer = Pointer::from_file(&temp_path)?;
        if downloaded_pointer.oid != pointer.oid {
            std::fs::remove_file(&temp_path).ok();
            let err: Box<dyn std::error::Error> =
                format!("hash mismatch for {}", file_hint).into();
            return Err(err);
        }

        // Read the downloaded content
        let data = std::fs::read(&temp_path)?;

        // Cache it
        if let Ok(cache) = Cache::new() {
            let _ = cache.put(&oid, &data);
        }

        // Clean up temp file
        std::fs::remove_file(&temp_path).ok();

        Ok::<Vec<u8>, Box<dyn std::error::Error>>(data)
    });

    match result {
        Ok(data) => {
            io::stdout().write_all(&data)?;
            io::stdout().flush()?;
        }
        Err(e) => {
            // Graceful degradation: output the pointer content + warning
            eprintln!(
                "gg lfs smudge: warning: download failed for {}: {}",
                file_hint, e
            );
            io::stdout().write_all(&content)?;
            io::stdout().flush()?;
        }
    }

    Ok(())
}
