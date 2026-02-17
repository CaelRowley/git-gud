//! Scanner for .gitattributes patterns
//!
//! Scans the repository for files matching LFS patterns defined in .gitattributes

use glob::Pattern;
use ignore::WalkBuilder;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScannerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid glob pattern: {0}")]
    InvalidPattern(String),

    #[error("Repository not found")]
    NoRepository,
}

/// A pattern from .gitattributes that marks files for LFS
#[derive(Debug, Clone)]
pub struct LfsPattern {
    /// The glob pattern
    pub pattern: String,
    /// The compiled pattern for matching
    compiled: Pattern,
}

impl LfsPattern {
    /// Create a new LFS pattern
    pub fn new(pattern: &str) -> Result<Self, ScannerError> {
        let compiled = Pattern::new(pattern)
            .map_err(|e| ScannerError::InvalidPattern(format!("{}: {}", pattern, e)))?;

        Ok(Self {
            pattern: pattern.to_string(),
            compiled,
        })
    }

    /// Check if a path matches this pattern
    pub fn matches(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.compiled.matches(&path_str)
            || self.compiled.matches(path.file_name().unwrap_or_default().to_string_lossy().as_ref())
    }
}

/// Scanner for finding LFS-tracked files
#[derive(Debug)]
pub struct Scanner {
    /// The repository root
    repo_root: PathBuf,
    /// Patterns that mark files for LFS
    patterns: Vec<LfsPattern>,
}

impl Scanner {
    /// Create a new scanner for the given repository
    pub fn new<P: AsRef<Path>>(repo_root: P) -> Result<Self, ScannerError> {
        let repo_root = repo_root.as_ref().to_path_buf();

        if !repo_root.join(".git").exists() {
            return Err(ScannerError::NoRepository);
        }

        let mut scanner = Self {
            repo_root,
            patterns: Vec::new(),
        };

        scanner.load_patterns()?;
        Ok(scanner)
    }

    /// Load LFS patterns from .gitattributes
    pub fn load_patterns(&mut self) -> Result<(), ScannerError> {
        self.patterns.clear();

        let gitattributes = self.repo_root.join(".gitattributes");
        if !gitattributes.exists() {
            return Ok(());
        }

        let file = File::open(&gitattributes)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse .gitattributes line: pattern attr1 attr2 ...
            // LFS files have: filter=gg-lfs diff=gg-lfs merge=gg-lfs -text
            // Also accept old filter=lfs for backwards compatibility
            if line.contains("filter=gg-lfs") || line.contains("filter=lfs") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(pattern) = parts.first() {
                    if let Ok(lfs_pattern) = LfsPattern::new(pattern) {
                        self.patterns.push(lfs_pattern);
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if a file path matches any LFS pattern
    pub fn is_lfs_file(&self, path: &Path) -> bool {
        for pattern in &self.patterns {
            if pattern.matches(path) {
                return true;
            }
        }
        false
    }

    /// Get all patterns
    pub fn patterns(&self) -> &[LfsPattern] {
        &self.patterns
    }

    /// Add a pattern to .gitattributes
    pub fn add_pattern(&mut self, pattern: &str) -> Result<(), ScannerError> {
        let gitattributes = self.repo_root.join(".gitattributes");

        // Check if pattern already exists (accept both old and new filter name)
        if gitattributes.exists() {
            let content = fs::read_to_string(&gitattributes)?;
            for line in content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.first() == Some(&pattern)
                    && (line.contains("filter=gg-lfs") || line.contains("filter=lfs"))
                {
                    // Pattern already exists
                    return Ok(());
                }
            }
        }

        // Append the pattern with new filter name
        let line = format!("{} filter=gg-lfs diff=gg-lfs merge=gg-lfs -text\n", pattern);
        let mut content = if gitattributes.exists() {
            let existing = fs::read_to_string(&gitattributes)?;
            if existing.ends_with('\n') {
                existing
            } else {
                format!("{}\n", existing)
            }
        } else {
            String::new()
        };

        content.push_str(&line);
        fs::write(&gitattributes, content)?;

        // Reload patterns
        self.load_patterns()?;

        Ok(())
    }

    /// Remove a pattern from .gitattributes
    pub fn remove_pattern(&mut self, pattern: &str) -> Result<bool, ScannerError> {
        let gitattributes = self.repo_root.join(".gitattributes");

        if !gitattributes.exists() {
            return Ok(false);
        }

        let content = fs::read_to_string(&gitattributes)?;
        let mut new_lines = Vec::new();
        let mut removed = false;

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.first() == Some(&pattern)
                && (line.contains("filter=gg-lfs") || line.contains("filter=lfs"))
            {
                removed = true;
                continue;
            }
            new_lines.push(line);
        }

        if removed {
            let new_content = new_lines.join("\n");
            let new_content = if new_content.is_empty() {
                new_content
            } else {
                format!("{}\n", new_content)
            };
            fs::write(&gitattributes, new_content)?;
            self.load_patterns()?;
        }

        Ok(removed)
    }

    /// Scan the repository for files matching LFS patterns.
    /// Respects .gitignore, .git/info/exclude, and global git excludes.
    pub fn scan_files(&self) -> Result<Vec<PathBuf>, ScannerError> {
        let mut files = Vec::new();

        for entry in WalkBuilder::new(&self.repo_root)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build()
        {
            let entry = entry.map_err(|e| {
                ScannerError::Io(io::Error::new(io::ErrorKind::Other, e))
            })?;
            if !entry.path().is_file() {
                continue;
            }
            if let Ok(rel) = entry.path().strip_prefix(&self.repo_root) {
                if self.is_lfs_file(rel) {
                    files.push(entry.into_path());
                }
            }
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lfs_pattern_simple() {
        let pattern = LfsPattern::new("*.psd").unwrap();
        assert!(pattern.matches(Path::new("image.psd")));
        assert!(pattern.matches(Path::new("assets/image.psd")));
        assert!(!pattern.matches(Path::new("image.png")));
    }

    #[test]
    fn test_lfs_pattern_directory() {
        let pattern = LfsPattern::new("assets/*").unwrap();
        assert!(pattern.matches(Path::new("assets/image.psd")));
        assert!(!pattern.matches(Path::new("src/main.rs")));
    }
}
