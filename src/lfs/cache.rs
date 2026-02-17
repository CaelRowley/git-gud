//! Local cache for LFS objects
//!
//! Caches downloaded LFS objects locally to avoid re-downloading.
//! Location: ~/.cache/gg-lfs/<sha256-prefix>/<sha256>

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum CacheError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Cache directory not found")]
    NoCacheDir,

    #[error("Object not in cache: {0}")]
    NotFound(String),
}

/// Local cache for LFS objects
#[derive(Debug)]
pub struct Cache {
    /// Root directory for the cache
    root: PathBuf,
}

#[allow(dead_code)]
impl Cache {
    /// Create a new cache with default location (~/.cache/gg-lfs)
    pub fn new() -> Result<Self, CacheError> {
        let cache_dir = dirs::cache_dir().ok_or(CacheError::NoCacheDir)?;
        let root = cache_dir.join("gg-lfs");

        fs::create_dir_all(&root)?;

        Ok(Self { root })
    }

    /// Create a cache at a specific location
    pub fn with_root<P: AsRef<Path>>(root: P) -> Result<Self, CacheError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// Get the path for a cached object
    fn object_path(&self, oid: &str) -> PathBuf {
        // Use first 2 chars as subdirectory for better filesystem performance
        let prefix = &oid[..2.min(oid.len())];
        self.root.join(prefix).join(oid)
    }

    /// Check if an object is in the cache
    pub fn contains(&self, oid: &str) -> bool {
        self.object_path(oid).exists()
    }

    /// Get the path to a cached object, if it exists
    pub fn get(&self, oid: &str) -> Option<PathBuf> {
        let path = self.object_path(oid);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Store content in the cache
    pub fn put(&self, oid: &str, content: &[u8]) -> Result<PathBuf, CacheError> {
        let path = self.object_path(oid);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = File::create(&path)?;
        file.write_all(content)?;
        file.flush()?;

        Ok(path)
    }

    /// Store a file in the cache (by copying)
    pub fn put_file<P: AsRef<Path>>(&self, oid: &str, source: P) -> Result<PathBuf, CacheError> {
        let path = self.object_path(oid);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::copy(source, &path)?;
        Ok(path)
    }

    /// Read content from the cache
    pub fn read(&self, oid: &str) -> Result<Vec<u8>, CacheError> {
        let path = self.object_path(oid);

        if !path.exists() {
            return Err(CacheError::NotFound(oid.to_string()));
        }

        let mut file = File::open(&path)?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;

        Ok(content)
    }

    /// Copy from cache to destination
    pub fn copy_to<P: AsRef<Path>>(&self, oid: &str, dest: P) -> Result<u64, CacheError> {
        let path = self.object_path(oid);

        if !path.exists() {
            return Err(CacheError::NotFound(oid.to_string()));
        }

        let bytes = fs::copy(&path, dest)?;
        Ok(bytes)
    }

    /// Remove an object from the cache
    pub fn remove(&self, oid: &str) -> Result<bool, CacheError> {
        let path = self.object_path(oid);

        if path.exists() {
            fs::remove_file(&path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get total size of cached objects in bytes
    pub fn size(&self) -> Result<u64, CacheError> {
        let mut total = 0;

        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                for file_entry in fs::read_dir(&path)? {
                    let file_entry = file_entry?;
                    if file_entry.path().is_file() {
                        total += file_entry.metadata()?.len();
                    }
                }
            }
        }

        Ok(total)
    }

    /// Count number of cached objects
    pub fn count(&self) -> Result<usize, CacheError> {
        let mut count = 0;

        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                for file_entry in fs::read_dir(&path)? {
                    let file_entry = file_entry?;
                    if file_entry.path().is_file() {
                        count += 1;
                    }
                }
            }
        }

        Ok(count)
    }

    /// Clear the entire cache
    pub fn clear(&self) -> Result<usize, CacheError> {
        let count = self.count()?;

        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                fs::remove_dir_all(&path)?;
            } else if path.is_file() {
                fs::remove_file(&path)?;
            }
        }

        Ok(count)
    }

    /// Prune objects not accessed in the given number of days
    pub fn prune(&self, days: u32) -> Result<usize, CacheError> {
        use std::time::{Duration, SystemTime};

        let cutoff = SystemTime::now() - Duration::from_secs(days as u64 * 24 * 60 * 60);
        let mut pruned = 0;

        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                for file_entry in fs::read_dir(&path)? {
                    let file_entry = file_entry?;
                    let file_path = file_entry.path();

                    if file_path.is_file() {
                        if let Ok(metadata) = file_entry.metadata() {
                            if let Ok(accessed) = metadata.accessed() {
                                if accessed < cutoff {
                                    fs::remove_file(&file_path)?;
                                    pruned += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(pruned)
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new().expect("Failed to create default cache")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_put_and_get() {
        let temp = TempDir::new().unwrap();
        let cache = Cache::with_root(temp.path()).unwrap();

        let oid = "4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393";
        let content = b"Hello, World!";

        cache.put(oid, content).unwrap();

        assert!(cache.contains(oid));
        assert_eq!(cache.read(oid).unwrap(), content);
    }

    #[test]
    fn test_cache_not_found() {
        let temp = TempDir::new().unwrap();
        let cache = Cache::with_root(temp.path()).unwrap();

        assert!(!cache.contains("nonexistent"));
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_cache_remove() {
        let temp = TempDir::new().unwrap();
        let cache = Cache::with_root(temp.path()).unwrap();

        let oid = "abc123def456";
        cache.put(oid, b"test").unwrap();

        assert!(cache.contains(oid));
        assert!(cache.remove(oid).unwrap());
        assert!(!cache.contains(oid));
    }

    #[test]
    fn test_cache_size_and_count() {
        let temp = TempDir::new().unwrap();
        let cache = Cache::with_root(temp.path()).unwrap();

        cache.put("oid1", b"hello").unwrap();
        cache.put("oid2", b"world!").unwrap();

        assert_eq!(cache.count().unwrap(), 2);
        assert_eq!(cache.size().unwrap(), 11); // 5 + 6 bytes
    }

    #[test]
    fn test_cache_put_file() {
        let temp = TempDir::new().unwrap();
        let cache = Cache::with_root(temp.path().join("cache")).unwrap();

        // Write a source file
        let source = temp.path().join("source.bin");
        fs::write(&source, b"file content here").unwrap();

        let oid = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab";
        let cached_path = cache.put_file(oid, &source).unwrap();

        assert!(cached_path.exists());
        assert!(cache.contains(oid));
        assert_eq!(cache.read(oid).unwrap(), b"file content here");
    }

    #[test]
    fn test_cache_copy_to() {
        let temp = TempDir::new().unwrap();
        let cache = Cache::with_root(temp.path().join("cache")).unwrap();

        let oid = "abc123";
        cache.put(oid, b"cached data").unwrap();

        let dest = temp.path().join("restored.bin");
        let bytes = cache.copy_to(oid, &dest).unwrap();

        assert_eq!(bytes, 11);
        assert_eq!(fs::read(&dest).unwrap(), b"cached data");
    }

    #[test]
    fn test_cache_copy_to_not_found() {
        let temp = TempDir::new().unwrap();
        let cache = Cache::with_root(temp.path()).unwrap();

        let dest = temp.path().join("out.bin");
        let result = cache.copy_to("nonexistent", &dest);
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_clear() {
        let temp = TempDir::new().unwrap();
        let cache = Cache::with_root(temp.path()).unwrap();

        cache.put("oid1", b"one").unwrap();
        cache.put("oid2", b"two").unwrap();
        cache.put("oid3", b"three").unwrap();

        assert_eq!(cache.count().unwrap(), 3);

        let cleared = cache.clear().unwrap();
        assert_eq!(cleared, 3);
        assert_eq!(cache.count().unwrap(), 0);
    }

    #[test]
    fn test_cache_clear_empty() {
        let temp = TempDir::new().unwrap();
        let cache = Cache::with_root(temp.path()).unwrap();

        let cleared = cache.clear().unwrap();
        assert_eq!(cleared, 0);
    }

    #[test]
    fn test_cache_prune_keeps_recent() {
        let temp = TempDir::new().unwrap();
        let cache = Cache::with_root(temp.path()).unwrap();

        cache.put("oid1", b"recent").unwrap();

        // Pruning with 30 days should keep a just-created file
        let pruned = cache.prune(30).unwrap();
        assert_eq!(pruned, 0);
        assert!(cache.contains("oid1"));
    }

    #[test]
    fn test_cache_read_not_found() {
        let temp = TempDir::new().unwrap();
        let cache = Cache::with_root(temp.path()).unwrap();

        let result = cache.read("nonexistent");
        assert!(matches!(result, Err(CacheError::NotFound(_))));
    }

    #[test]
    fn test_cache_remove_nonexistent() {
        let temp = TempDir::new().unwrap();
        let cache = Cache::with_root(temp.path()).unwrap();

        let removed = cache.remove("nonexistent").unwrap();
        assert!(!removed);
    }

    #[test]
    fn test_cache_overwrite_existing() {
        let temp = TempDir::new().unwrap();
        let cache = Cache::with_root(temp.path()).unwrap();

        let oid = "abc123";
        cache.put(oid, b"first").unwrap();
        cache.put(oid, b"second").unwrap();

        assert_eq!(cache.read(oid).unwrap(), b"second");
        assert_eq!(cache.count().unwrap(), 1);
    }
}
