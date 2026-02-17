//! Clean filter for LFS
//!
//! Invoked by git during `git add` via the filter driver:
//!   filter.gg-lfs.clean = "gg lfs clean %f"
//!
//! Reads file content from stdin, outputs pointer text to stdout.
//! Caches the original content locally (no network access).
//! Streams content to avoid loading large files into memory.

use crate::lfs::pointer::MAX_POINTER_SIZE;
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
    let stdin = io::stdin();
    let mut reader = stdin.lock();

    // Read a header chunk (MAX_POINTER_SIZE + 1) to determine if this is already a pointer
    let mut header = vec![0u8; MAX_POINTER_SIZE + 1];
    let header_len = read_exact_or_eof(&mut reader, &mut header)?;
    header.truncate(header_len);

    // If the entire content fits in the header and parses as a pointer, pass through unchanged
    if header_len <= MAX_POINTER_SIZE {
        if let Ok(_pointer) = Pointer::parse_content(io::BufReader::new(header.as_slice())) {
            io::stdout().write_all(&header)?;
            io::stdout().flush()?;
            return Ok(());
        }
    }

    // Not a pointer — stream through hasher + cache file
    // Build a cache path (best-effort)
    let cache = Cache::new().ok();
    let temp_dir = cache.as_ref().map(|c| c.temp_dir());
    let temp_path = temp_dir.as_ref().and_then(|d| {
        std::fs::create_dir_all(d).ok()?;
        Some(d.join(format!("clean-{}", std::process::id())))
    });

    // Chain header bytes with remaining stdin into a single reader
    let remaining = io::Cursor::new(Vec::new()).chain(reader);
    let chained = io::Cursor::new(header).chain(remaining);

    let pointer = Pointer::from_reader(chained, temp_path.as_deref())?;
    let oid = pointer.sha256().to_string();

    // Move temp file to cache (best-effort)
    if let (Some(cache), Some(temp)) = (&cache, &temp_path) {
        let _ = cache.put_file(&oid, temp);
        let _ = std::fs::remove_file(temp);
    }

    // Write pointer text to stdout
    let pointer_text = format!("{}", pointer);
    io::stdout().write_all(pointer_text.as_bytes())?;
    io::stdout().flush()?;

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
