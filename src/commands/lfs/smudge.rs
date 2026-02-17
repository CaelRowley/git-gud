//! Smudge filter for LFS
//!
//! Invoked by git during `git checkout` via the filter driver:
//!   filter.lfs.smudge = "gg lfs smudge %f"
//!
//! Reads pointer text from stdin, outputs real file content to stdout.
//! Checks local cache first, falls back to S3 download on cache miss.

use crate::lfs::pointer::MAX_POINTER_SIZE;
use crate::lfs::storage;
use crate::lfs::{Cache, LfsConfig, Pointer};
use clap::Args;
use std::io::{self, Read, Write};

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
    // Skip smudge if GG_LFS_SKIP_SMUDGE=1 (useful for CI)
    if std::env::var("GG_LFS_SKIP_SMUDGE").unwrap_or_default() == "1" {
        io::copy(&mut io::stdin(), &mut io::stdout())?;
        return Ok(());
    }

    // Read header to determine if this is a pointer (bounded read to avoid OOM)
    let mut header = vec![0u8; MAX_POINTER_SIZE + 1];
    let header_len = read_exact_or_eof(&mut io::stdin().lock(), &mut header)?;
    header.truncate(header_len);

    // Try parsing as pointer — only possible if content fits in header
    if header_len <= MAX_POINTER_SIZE {
        if let Ok(pointer) = Pointer::parse_content(io::BufReader::new(header.as_slice())) {
            // It's a pointer — download the real content
            return download_and_output(&pointer, &args, &header);
        }
    }

    // Not a pointer — stream through unchanged
    io::stdout().write_all(&header)?;
    io::copy(&mut io::stdin().lock(), &mut io::stdout())?;
    io::stdout().flush()?;
    Ok(())
}

/// Download the real content for a pointer and write to stdout
fn download_and_output(
    pointer: &Pointer,
    args: &SmudgeArgs,
    pointer_bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let oid = pointer.sha256().to_string();

    // Check local cache first — stream directly to stdout
    if let Ok(cache) = Cache::new() {
        if let Some(cached_path) = cache.get(&oid) {
            let mut file = std::fs::File::open(&cached_path)?;
            io::copy(&mut file, &mut io::stdout())?;
            io::stdout().flush()?;
            return Ok(());
        }
    }

    // Cache miss — try S3 download
    let file_hint = args.file.as_deref().unwrap_or("<unknown>");

    let repo = match git2::Repository::discover(".") {
        Ok(r) => r,
        Err(_) => {
            eprintln!(
                "gg lfs smudge: warning: cannot find repository for {}, outputting pointer",
                file_hint
            );
            io::stdout().write_all(pointer_bytes)?;
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
            io::stdout().write_all(pointer_bytes)?;
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
            io::stdout().write_all(pointer_bytes)?;
            io::stdout().flush()?;
            return Ok(());
        }
    };

    // Need async runtime for S3 download
    let rt = tokio::runtime::Runtime::new()?;
    let result = rt.block_on(async {
        let storage = storage::create_storage(&config).await?;

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

        // Cache the downloaded file
        if let Ok(cache) = Cache::new() {
            let _ = cache.put_file(&oid, &temp_path);
        }

        // Stream temp file to stdout instead of reading into memory
        let mut file = std::fs::File::open(&temp_path)?;
        io::copy(&mut file, &mut io::stdout())?;
        io::stdout().flush()?;

        // Clean up temp file
        std::fs::remove_file(&temp_path).ok();

        Ok::<(), Box<dyn std::error::Error>>(())
    });

    if let Err(e) = result {
        // Graceful degradation: output the pointer content + warning
        eprintln!(
            "gg lfs smudge: warning: download failed for {}: {}",
            file_hint, e
        );
        io::stdout().write_all(pointer_bytes)?;
        io::stdout().flush()?;
    }

    Ok(())
}

/// Read up to `buf.len()` bytes, returning the actual number read.
/// Unlike `read_exact`, does not error on EOF.
fn read_exact_or_eof<R: Read>(reader: &mut R, buf: &mut [u8]) -> io::Result<usize> {
    let mut total = 0;
    while total < buf.len() {
        match reader.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(total)
}
