//! LFS Pointer file handling
//!
//! Pointer files are small text files that replace large files in git.
//! Format is compatible with git-lfs:
//!
//! ```text
//! version https://git-lfs.github.com/spec/v1
//! oid sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393
//! size 12345
//! ```

use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use thiserror::Error;

/// The version URL for LFS pointer files (git-lfs compatible)
pub const LFS_VERSION: &str = "https://git-lfs.github.com/spec/v1";

/// Maximum size for a pointer file (per LFS spec)
pub const MAX_POINTER_SIZE: usize = 1024;

#[derive(Error, Debug)]
pub enum PointerError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Invalid pointer file: {0}")]
    InvalidFormat(String),

    #[error("Pointer file too large (max {MAX_POINTER_SIZE} bytes)")]
    TooLarge,

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid OID format: {0}")]
    InvalidOid(String),
}

/// Represents an LFS pointer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pointer {
    /// The version URL (always LFS_VERSION for compatibility)
    pub version: String,
    /// The object ID (sha256:hexdigest)
    pub oid: String,
    /// The original file size in bytes
    pub size: u64,
}

#[allow(dead_code)]
impl Pointer {
    /// Create a new pointer from file content (streaming — no full read into memory)
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, PointerError> {
        let file = File::open(path.as_ref())?;
        Self::from_reader(file, None)
    }

    /// Create a pointer by streaming content from a reader.
    /// Optionally writes the content to `cache_path` while hashing.
    pub fn from_reader<R: Read>(
        mut reader: R,
        cache_path: Option<&Path>,
    ) -> Result<Self, PointerError> {
        let mut hasher = Sha256::new();
        let mut size: u64 = 0;
        let mut cache_file = cache_path.map(|p| File::create(p)).transpose()?;
        let mut buf = [0u8; 64 * 1024];

        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            size += n as u64;
            if let Some(ref mut f) = cache_file {
                f.write_all(&buf[..n])?;
            }
        }

        let hash = hasher.finalize();
        let oid = format!("sha256:{:x}", hash);

        Ok(Self {
            version: LFS_VERSION.to_string(),
            oid,
            size,
        })
    }

    /// Create a pointer from raw bytes (for hashing content not yet on disk)
    pub fn from_bytes(content: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash = hasher.finalize();
        let oid = format!("sha256:{:x}", hash);

        Self {
            version: LFS_VERSION.to_string(),
            oid,
            size: content.len() as u64,
        }
    }

    /// Parse a pointer from a pointer file
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<Self, PointerError> {
        let path = path.as_ref();
        let metadata = fs::metadata(path)?;

        if metadata.len() > MAX_POINTER_SIZE as u64 {
            return Err(PointerError::TooLarge);
        }

        let file = File::open(path)?;
        let reader = BufReader::new(file);

        Self::parse_content(reader)
    }

    /// Parse pointer content from a reader
    pub fn parse_content<R: BufRead>(reader: R) -> Result<Self, PointerError> {
        let mut version = None;
        let mut oid = None;
        let mut size = None;

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() != 2 {
                return Err(PointerError::InvalidFormat(format!(
                    "Invalid line format: {}",
                    line
                )));
            }

            let (key, value) = (parts[0], parts[1]);

            match key {
                "version" => version = Some(value.to_string()),
                "oid" => {
                    // Validate OID format
                    if !value.starts_with("sha256:") {
                        return Err(PointerError::InvalidOid(value.to_string()));
                    }
                    let hex_part = &value[7..];
                    if hex_part.len() != 64 || !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
                        return Err(PointerError::InvalidOid(value.to_string()));
                    }
                    oid = Some(value.to_string());
                }
                "size" => {
                    size = Some(value.parse::<u64>().map_err(|_| {
                        PointerError::InvalidFormat(format!("Invalid size: {}", value))
                    })?);
                }
                _ => {
                    // Ignore unknown keys (allows for extensions)
                }
            }
        }

        Ok(Self {
            version: version.ok_or_else(|| PointerError::MissingField("version".to_string()))?,
            oid: oid.ok_or_else(|| PointerError::MissingField("oid".to_string()))?,
            size: size.ok_or_else(|| PointerError::MissingField("size".to_string()))?,
        })
    }

    /// Write the pointer to a file
    pub fn write<P: AsRef<Path>>(&self, path: P) -> Result<(), PointerError> {
        let content = self.to_string();
        let mut file = File::create(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    /// Get the SHA256 hash (without the sha256: prefix)
    pub fn sha256(&self) -> &str {
        self.oid
            .strip_prefix("sha256:")
            .unwrap_or(&self.oid)
    }

    /// Check if a file is a pointer file (by examining its content)
    pub fn is_pointer_file<P: AsRef<Path>>(path: P) -> bool {
        let path = path.as_ref();

        // Quick size check first
        if let Ok(metadata) = fs::metadata(path) {
            if metadata.len() > MAX_POINTER_SIZE as u64 {
                return false;
            }
        } else {
            return false;
        }

        // Try to parse as pointer
        Self::parse(path).is_ok()
    }
}

impl std::fmt::Display for Pointer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Version must come first, then alphabetically sorted keys
        writeln!(f, "version {}", self.version)?;
        writeln!(f, "oid {}", self.oid)?;
        writeln!(f, "size {}", self.size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_pointer_from_bytes() {
        let content = b"Hello, World!";
        let pointer = Pointer::from_bytes(content);

        assert_eq!(pointer.version, LFS_VERSION);
        assert!(pointer.oid.starts_with("sha256:"));
        assert_eq!(pointer.size, 13);
    }

    #[test]
    fn test_pointer_parse_valid() {
        let content = "version https://git-lfs.github.com/spec/v1\noid sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393\nsize 12345\n";
        let reader = Cursor::new(content);
        let pointer = Pointer::parse_content(reader).unwrap();

        assert_eq!(pointer.version, LFS_VERSION);
        assert_eq!(
            pointer.oid,
            "sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393"
        );
        assert_eq!(pointer.size, 12345);
    }

    #[test]
    fn test_pointer_parse_invalid_oid() {
        let content = "version https://git-lfs.github.com/spec/v1\noid md5:abc123\nsize 100\n";
        let reader = Cursor::new(content);
        let result = Pointer::parse_content(reader);

        assert!(matches!(result, Err(PointerError::InvalidOid(_))));
    }

    #[test]
    fn test_pointer_to_string() {
        let pointer = Pointer {
            version: LFS_VERSION.to_string(),
            oid: "sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393"
                .to_string(),
            size: 12345,
        };

        let output = pointer.to_string();
        assert!(output.starts_with("version "));
        assert!(output.contains("oid sha256:"));
        assert!(output.contains("size 12345"));
        assert!(output.ends_with('\n'), "Pointer display must end with newline");
    }

    #[test]
    fn test_pointer_sha256() {
        let pointer = Pointer {
            version: LFS_VERSION.to_string(),
            oid: "sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393"
                .to_string(),
            size: 100,
        };

        assert_eq!(
            pointer.sha256(),
            "4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393"
        );
    }

    #[test]
    fn test_pointer_parse_missing_version() {
        let content = "oid sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393\nsize 100\n";
        let reader = Cursor::new(content);
        let result = Pointer::parse_content(reader);
        assert!(matches!(result, Err(PointerError::MissingField(_))));
    }

    #[test]
    fn test_pointer_parse_missing_oid() {
        let content = "version https://git-lfs.github.com/spec/v1\nsize 100\n";
        let reader = Cursor::new(content);
        let result = Pointer::parse_content(reader);
        assert!(matches!(result, Err(PointerError::MissingField(_))));
    }

    #[test]
    fn test_pointer_parse_missing_size() {
        let content = "version https://git-lfs.github.com/spec/v1\noid sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393\n";
        let reader = Cursor::new(content);
        let result = Pointer::parse_content(reader);
        assert!(matches!(result, Err(PointerError::MissingField(_))));
    }

    #[test]
    fn test_pointer_parse_empty_input() {
        let content = "";
        let reader = Cursor::new(content);
        let result = Pointer::parse_content(reader);
        assert!(matches!(result, Err(PointerError::MissingField(_))));
    }

    #[test]
    fn test_pointer_parse_ignores_unknown_keys() {
        let content = "version https://git-lfs.github.com/spec/v1\noid sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393\nsize 100\nextension some-extension\n";
        let reader = Cursor::new(content);
        let pointer = Pointer::parse_content(reader).unwrap();
        assert_eq!(pointer.size, 100);
    }

    #[test]
    fn test_pointer_parse_invalid_oid_short_hex() {
        let content = "version https://git-lfs.github.com/spec/v1\noid sha256:abcdef\nsize 100\n";
        let reader = Cursor::new(content);
        let result = Pointer::parse_content(reader);
        assert!(matches!(result, Err(PointerError::InvalidOid(_))));
    }

    #[test]
    fn test_pointer_parse_invalid_size() {
        let content = "version https://git-lfs.github.com/spec/v1\noid sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393\nsize notanumber\n";
        let reader = Cursor::new(content);
        let result = Pointer::parse_content(reader);
        assert!(matches!(result, Err(PointerError::InvalidFormat(_))));
    }

    #[test]
    fn test_pointer_write_and_parse_roundtrip() {
        let temp = tempfile::TempDir::new().unwrap();
        let file_path = temp.path().join("test.bin");

        // Create a pointer and write it
        let original = Pointer {
            version: LFS_VERSION.to_string(),
            oid: "sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393"
                .to_string(),
            size: 12345,
        };
        original.write(&file_path).unwrap();

        // Parse it back
        let parsed = Pointer::parse(&file_path).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_pointer_from_file_and_write_roundtrip() {
        let temp = tempfile::TempDir::new().unwrap();
        let real_file = temp.path().join("data.bin");
        let pointer_file = temp.path().join("data.ptr");

        // Write real content
        std::fs::write(&real_file, b"some binary content\x00\x01").unwrap();

        // Create pointer from file, write it, parse it back
        let pointer = Pointer::from_file(&real_file).unwrap();
        pointer.write(&pointer_file).unwrap();
        let parsed = Pointer::parse(&pointer_file).unwrap();

        assert_eq!(pointer, parsed);
    }

    #[test]
    fn test_is_pointer_file_with_pointer() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("ptr");
        let pointer = Pointer::from_bytes(b"test content");
        pointer.write(&path).unwrap();

        assert!(Pointer::is_pointer_file(&path));
    }

    #[test]
    fn test_is_pointer_file_with_real_content() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("real");
        std::fs::write(&path, b"this is not a pointer file").unwrap();

        assert!(!Pointer::is_pointer_file(&path));
    }

    #[test]
    fn test_is_pointer_file_nonexistent() {
        assert!(!Pointer::is_pointer_file("/tmp/does_not_exist_gg_test"));
    }

    #[test]
    fn test_is_pointer_file_too_large() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("large");
        let content = vec![b'x'; MAX_POINTER_SIZE + 1];
        std::fs::write(&path, &content).unwrap();

        assert!(!Pointer::is_pointer_file(&path));
    }

    #[test]
    fn test_pointer_parse_blank_lines_ignored() {
        let content = "\nversion https://git-lfs.github.com/spec/v1\n\noid sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393\n\nsize 100\n\n";
        let reader = Cursor::new(content);
        let pointer = Pointer::parse_content(reader).unwrap();
        assert_eq!(pointer.size, 100);
    }
}
