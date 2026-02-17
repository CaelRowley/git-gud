//! Long-running filter-process protocol for LFS
//!
//! Implements git's long-running filter-process protocol (gitattributes(5))
//! to handle clean/smudge in a single persistent process, avoiding per-file
//! process spawn + tokio runtime + S3 client initialization overhead.

use crate::lfs::pointer::MAX_POINTER_SIZE;
use crate::lfs::storage::{self, Storage};
use crate::lfs::{Cache, LfsConfig, Pointer};
use clap::Args;
use std::io::{self, BufWriter, Read, Write};
use std::path::Path;

/// Maximum data payload per pkt-line frame (65520 - 4 byte length prefix)
const PKT_MAX_DATA: usize = 65516;

#[derive(Args, Debug)]
pub struct FilterProcessArgs {}

/// Run the long-running filter process
pub fn run(_args: FilterProcessArgs) -> i32 {
    match run_inner() {
        Ok(_) => 0,
        Err(e) => {
            eprintln!("gg lfs filter-process: {}", e);
            1
        }
    }
}

// ── pkt-line protocol primitives ─────────────────────────────────────

enum PktLine {
    Data(Vec<u8>),
    Flush,
}

/// Read a single pkt-line frame. Returns None on EOF.
fn pkt_read<R: Read>(reader: &mut R) -> io::Result<Option<PktLine>> {
    let mut len_buf = [0u8; 4];
    match reader.read_exact(&mut len_buf) {
        Ok(_) => {}
        Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }

    let len_str = std::str::from_utf8(&len_buf)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid pkt-line length"))?;
    let len = usize::from_str_radix(len_str, 16)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid pkt-line hex"))?;

    if len == 0 {
        return Ok(Some(PktLine::Flush));
    }

    if len < 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "pkt-line length < 4",
        ));
    }

    let data_len = len - 4;
    let mut data = vec![0u8; data_len];
    reader.read_exact(&mut data)?;

    Ok(Some(PktLine::Data(data)))
}

/// Read pkt-line data frames until flush, collecting into a Vec.
/// Only use for small content (e.g., pointer text).
fn pkt_read_to_flush<R: Read>(reader: &mut R) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    loop {
        match pkt_read(reader)? {
            Some(PktLine::Data(data)) => buf.extend_from_slice(&data),
            Some(PktLine::Flush) | None => break,
        }
    }
    Ok(buf)
}

/// Write a text line as a pkt-line packet.
fn pkt_write<W: Write>(writer: &mut W, line: &str) -> io::Result<()> {
    write!(writer, "{:04x}{}", line.len() + 4, line)?;
    Ok(())
}

/// Write a flush packet and flush the underlying writer.
fn pkt_flush<W: Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all(b"0000")?;
    writer.flush()?;
    Ok(())
}

/// Write binary data as pkt-line frames (handles chunking).
fn pkt_write_data<W: Write>(writer: &mut W, data: &[u8]) -> io::Result<()> {
    for chunk in data.chunks(PKT_MAX_DATA) {
        write!(writer, "{:04x}", chunk.len() + 4)?;
        writer.write_all(chunk)?;
    }
    Ok(())
}

/// Stream a file's content as pkt-line frames.
fn pkt_stream_file<W: Write>(writer: &mut W, path: &Path) -> io::Result<()> {
    let mut file = std::fs::File::open(path)?;
    let mut buf = vec![0u8; PKT_MAX_DATA];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        write!(writer, "{:04x}", n + 4)?;
        writer.write_all(&buf[..n])?;
    }
    Ok(())
}

// ── PktLineReader: Read adapter over pkt-line stream ─────────────────

/// Presents a standard Read interface over pkt-line data frames.
/// Reads frames until a flush packet, then returns EOF (0).
struct PktLineReader<'a, R> {
    inner: &'a mut R,
    buf: Vec<u8>,
    pos: usize,
    done: bool,
}

impl<'a, R: Read> PktLineReader<'a, R> {
    fn new(inner: &'a mut R) -> Self {
        Self {
            inner,
            buf: Vec::new(),
            pos: 0,
            done: false,
        }
    }
}

impl<R: Read> Read for PktLineReader<'_, R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.done {
            return Ok(0);
        }
        if self.pos >= self.buf.len() {
            match pkt_read(self.inner)? {
                Some(PktLine::Data(data)) => {
                    self.buf = data;
                    self.pos = 0;
                }
                Some(PktLine::Flush) | None => {
                    self.done = true;
                    return Ok(0);
                }
            }
        }
        let available = self.buf.len() - self.pos;
        let n = out.len().min(available);
        out[..n].copy_from_slice(&self.buf[self.pos..self.pos + n]);
        self.pos += n;
        Ok(n)
    }
}

// ── Protocol handshake ───────────────────────────────────────────────

fn handshake<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read client identification
    let ident = match pkt_read(reader)? {
        Some(PktLine::Data(d)) => String::from_utf8_lossy(&d).trim().to_string(),
        _ => return Err("expected git-filter-client".into()),
    };
    if ident != "git-filter-client" {
        return Err(format!("expected 'git-filter-client', got '{}'", ident).into());
    }

    // Read version(s) until flush
    let mut got_v2 = false;
    loop {
        match pkt_read(reader)? {
            Some(PktLine::Data(d)) => {
                if String::from_utf8_lossy(&d).trim() == "version=2" {
                    got_v2 = true;
                }
            }
            _ => break,
        }
    }
    if !got_v2 {
        return Err("client did not offer version=2".into());
    }

    // Send server identification + version
    pkt_write(writer, "git-filter-server\n")?;
    pkt_write(writer, "version=2\n")?;
    pkt_flush(writer)?;

    // Read client capabilities until flush
    let mut client_caps = Vec::new();
    loop {
        match pkt_read(reader)? {
            Some(PktLine::Data(d)) => {
                let line = String::from_utf8_lossy(&d).trim().to_string();
                if let Some(cap) = line.strip_prefix("capability=") {
                    client_caps.push(cap.to_string());
                }
            }
            _ => break,
        }
    }

    // Respond with supported capabilities (intersection with client)
    if client_caps.iter().any(|c| c == "clean") {
        pkt_write(writer, "capability=clean\n")?;
    }
    if client_caps.iter().any(|c| c == "smudge") {
        pkt_write(writer, "capability=smudge\n")?;
    }
    pkt_flush(writer)?;

    Ok(())
}

// ── Main loop ────────────────────────────────────────────────────────

fn run_inner() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = BufWriter::new(stdout.lock());

    handshake(&mut reader, &mut writer)?;

    // Shared resources — initialized once, reused for all files
    let cache = Cache::new().ok();
    let repo = git2::Repository::discover(".")?;
    let repo_root = repo
        .workdir()
        .ok_or("Not a git repository with a working directory")?
        .to_path_buf();

    let rt = tokio::runtime::Runtime::new()?;
    let storage: Option<Box<dyn Storage>> = LfsConfig::load(&repo_root)
        .ok()
        .and_then(|config| rt.block_on(storage::create_storage(&config)).ok());

    let skip_smudge = std::env::var("GG_LFS_SKIP_SMUDGE").unwrap_or_default() == "1";

    loop {
        // Read command metadata until flush
        let mut command = String::new();
        let mut pathname = String::new();

        loop {
            match pkt_read(&mut reader)? {
                Some(PktLine::Data(d)) => {
                    let line = String::from_utf8_lossy(&d).trim().to_string();
                    if let Some(cmd) = line.strip_prefix("command=") {
                        command = cmd.to_string();
                    } else if let Some(path) = line.strip_prefix("pathname=") {
                        pathname = path.to_string();
                    }
                }
                Some(PktLine::Flush) => break,
                None => return Ok(()), // Git closed stdin — clean exit
            }
        }

        if command.is_empty() {
            return Ok(());
        }

        let result = match command.as_str() {
            "clean" => process_clean(&mut reader, &mut writer, &cache),
            "smudge" if skip_smudge => process_passthrough(&mut reader, &mut writer),
            "smudge" => process_smudge(
                &mut reader,
                &mut writer,
                &cache,
                storage.as_deref(),
                &rt,
                &repo_root,
                &pathname,
            ),
            _ => process_passthrough(&mut reader, &mut writer),
        };

        if let Err(e) = result {
            eprintln!(
                "gg lfs filter-process: error on {} ({}): {}",
                pathname, command, e
            );
            let _ = pkt_write(&mut writer, "status=error\n");
            let _ = pkt_flush(&mut writer);
            let _ = pkt_flush(&mut writer);
        }
    }
}

// ── Command handlers ─────────────────────────────────────────────────

/// Pass content through unchanged.
fn process_passthrough<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = pkt_read_to_flush(reader)?;
    pkt_write(writer, "status=success\n")?;
    pkt_flush(writer)?;
    pkt_write_data(writer, &content)?;
    pkt_flush(writer)?;
    pkt_flush(writer)?;
    Ok(())
}

/// Clean filter: convert file content to pointer text.
/// Streams through hasher + cache file to handle large files without OOM.
fn process_clean<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    cache: &Option<Cache>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut pkt_reader = PktLineReader::new(reader);

    // Read header to check if already a pointer
    let mut header = vec![0u8; MAX_POINTER_SIZE + 1];
    let header_len = read_exact_or_eof(&mut pkt_reader, &mut header)?;
    header.truncate(header_len);

    // If fits in header and is a pointer, pass through unchanged
    if header_len <= MAX_POINTER_SIZE && pkt_reader.done {
        if Pointer::parse_content(io::BufReader::new(header.as_slice())).is_ok() {
            pkt_write(writer, "status=success\n")?;
            pkt_flush(writer)?;
            pkt_write_data(writer, &header)?;
            pkt_flush(writer)?;
            pkt_flush(writer)?;
            return Ok(());
        }
    }

    // Not a pointer — stream through hasher + temp file for caching
    let temp_path = cache.as_ref().and_then(|c| {
        let dir = c.temp_dir();
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir.join(format!("filter-clean-{}", std::process::id())))
    });

    let chained = io::Cursor::new(header).chain(pkt_reader);
    let pointer = Pointer::from_reader(chained, temp_path.as_deref())?;
    let oid = pointer.sha256().to_string();

    // Cache the content
    if let (Some(cache), Some(ref temp)) = (cache, &temp_path) {
        let _ = cache.put_file(&oid, temp);
        let _ = std::fs::remove_file(temp);
    }

    // Write pointer text as response
    let pointer_text = pointer.to_string();
    pkt_write(writer, "status=success\n")?;
    pkt_flush(writer)?;
    pkt_write_data(writer, pointer_text.as_bytes())?;
    pkt_flush(writer)?;
    pkt_flush(writer)?;

    Ok(())
}

/// Smudge filter: convert pointer text to real file content.
/// Input is always small (pointer text). Output may be large (streamed).
fn process_smudge<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    cache: &Option<Cache>,
    storage: Option<&dyn Storage>,
    rt: &tokio::runtime::Runtime,
    repo_root: &Path,
    pathname: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = pkt_read_to_flush(reader)?;

    // Try to parse as pointer
    let pointer = match Pointer::parse_content(io::BufReader::new(content.as_slice())) {
        Ok(p) => p,
        Err(_) => {
            // Not a pointer — pass through unchanged
            pkt_write(writer, "status=success\n")?;
            pkt_flush(writer)?;
            pkt_write_data(writer, &content)?;
            pkt_flush(writer)?;
            pkt_flush(writer)?;
            return Ok(());
        }
    };

    let oid = pointer.sha256().to_string();

    // Check cache first — stream directly
    if let Some(cache) = cache {
        if let Some(cached_path) = cache.get(&oid) {
            pkt_write(writer, "status=success\n")?;
            pkt_flush(writer)?;
            pkt_stream_file(writer, &cached_path)?;
            pkt_flush(writer)?;
            pkt_flush(writer)?;
            return Ok(());
        }
    }

    // Cache miss — download from storage
    let storage = match storage {
        Some(s) => s,
        None => {
            eprintln!(
                "gg lfs filter-process: warning: no storage for {}, outputting pointer",
                pathname
            );
            pkt_write(writer, "status=success\n")?;
            pkt_flush(writer)?;
            pkt_write_data(writer, &content)?;
            pkt_flush(writer)?;
            pkt_flush(writer)?;
            return Ok(());
        }
    };

    let temp_dir = repo_root.join(".gg").join("tmp");
    std::fs::create_dir_all(&temp_dir)?;
    let temp_path = temp_dir.join(&oid);

    rt.block_on(async { storage.download(&oid, &temp_path).await })?;

    // Verify hash
    let downloaded_pointer = Pointer::from_file(&temp_path)?;
    if downloaded_pointer.oid != pointer.oid {
        std::fs::remove_file(&temp_path).ok();
        return Err(format!("hash mismatch for {}", pathname).into());
    }

    // Cache the downloaded file
    if let Some(cache) = cache {
        let _ = cache.put_file(&oid, &temp_path);
    }

    // Stream to output
    pkt_write(writer, "status=success\n")?;
    pkt_flush(writer)?;
    pkt_stream_file(writer, &temp_path)?;
    pkt_flush(writer)?;
    pkt_flush(writer)?;

    std::fs::remove_file(&temp_path).ok();

    Ok(())
}

/// Read up to `buf.len()` bytes without erroring on EOF.
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
