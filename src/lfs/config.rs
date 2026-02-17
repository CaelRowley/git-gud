//! LFS configuration
//!
//! Configuration is stored in .gg/lfs.toml in the repository root.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("TOML serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),

    #[error("Configuration not found at {0}")]
    NotFound(PathBuf),

    #[error("Invalid configuration: {0}")]
    Invalid(String),

    #[error("Repository not found")]
    NoRepository,
}

/// Storage provider type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StorageProvider {
    S3,
    // Future: Gcs, Azure, etc.
}

impl Default for StorageProvider {
    fn default() -> Self {
        Self::S3
    }
}

/// Inline credential configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialsConfig {
    pub access_key_id: String,
    pub secret_access_key: String,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage provider (currently only "s3")
    #[serde(default)]
    pub provider: StorageProvider,

    /// S3 bucket name
    pub bucket: String,

    /// AWS region
    #[serde(default = "default_region")]
    pub region: String,

    /// Optional prefix for object keys
    #[serde(default)]
    pub prefix: Option<String>,

    /// Optional custom endpoint (for S3-compatible services like MinIO)
    #[serde(default)]
    pub endpoint: Option<String>,

    /// Optional inline credentials (alternative to env vars / ~/.aws/credentials)
    #[serde(default)]
    pub credentials: Option<CredentialsConfig>,
}

fn default_region() -> String {
    "us-east-1".to_string()
}

/// Main LFS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfsConfig {
    /// Storage configuration
    pub storage: StorageConfig,
}

#[allow(dead_code)]
impl LfsConfig {
    /// Find and load configuration from repository
    pub fn load<P: AsRef<Path>>(repo_root: P) -> Result<Self, ConfigError> {
        let config_path = Self::config_path(repo_root.as_ref());

        if !config_path.exists() {
            return Err(ConfigError::NotFound(config_path));
        }

        let content = fs::read_to_string(&config_path)?;
        let config: LfsConfig = toml::from_str(&content)?;

        config.validate()?;
        Ok(config)
    }

    /// Save configuration to repository
    pub fn save<P: AsRef<Path>>(&self, repo_root: P) -> Result<(), ConfigError> {
        let config_dir = repo_root.as_ref().join(".gg");
        fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("lfs.toml");
        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;

        Ok(())
    }

    /// Get the config file path for a repository
    pub fn config_path(repo_root: &Path) -> PathBuf {
        repo_root.join(".gg").join("lfs.toml")
    }

    /// Check if configuration exists
    pub fn exists<P: AsRef<Path>>(repo_root: P) -> bool {
        Self::config_path(repo_root.as_ref()).exists()
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.storage.bucket.is_empty() {
            return Err(ConfigError::Invalid("bucket cannot be empty".to_string()));
        }

        if self.storage.region.is_empty() {
            return Err(ConfigError::Invalid("region cannot be empty".to_string()));
        }

        Ok(())
    }

    /// Create a default/template configuration
    pub fn template() -> Self {
        Self {
            storage: StorageConfig {
                provider: StorageProvider::S3,
                bucket: "my-lfs-bucket".to_string(),
                region: "us-east-1".to_string(),
                prefix: Some("lfs/".to_string()),
                endpoint: None,
                credentials: None,
            },
        }
    }

    /// Generate template TOML content with comments
    pub fn template_toml() -> String {
        r#"# gg-lfs Configuration
# See: https://github.com/yourusername/git-gud

[storage]
# Storage provider: "s3" (more coming soon)
provider = "s3"

# S3 bucket name (required)
bucket = "my-lfs-bucket"

# AWS region (default: us-east-1)
region = "us-east-1"

# Optional prefix for object keys
# prefix = "project-name/"

# Optional custom endpoint for S3-compatible services (MinIO, DigitalOcean Spaces, etc.)
# endpoint = "https://nyc3.digitaloceanspaces.com"

# Credentials (optional - can also use env vars or ~/.aws/credentials)
# [storage.credentials]
# access_key_id = "AKIA..."
# secret_access_key = "..."
"#
        .to_string()
    }

    /// Write a template configuration file
    pub fn write_template<P: AsRef<Path>>(repo_root: P) -> Result<PathBuf, ConfigError> {
        let config_dir = repo_root.as_ref().join(".gg");
        fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("lfs.toml");
        fs::write(&config_path, Self::template_toml())?;

        Ok(config_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_template() {
        let config = LfsConfig::template();
        assert_eq!(config.storage.provider, StorageProvider::S3);
        assert!(!config.storage.bucket.is_empty());
    }

    #[test]
    fn test_config_save_and_load() {
        let temp = TempDir::new().unwrap();
        let config = LfsConfig::template();

        config.save(temp.path()).unwrap();

        let loaded = LfsConfig::load(temp.path()).unwrap();
        assert_eq!(loaded.storage.bucket, config.storage.bucket);
        assert_eq!(loaded.storage.region, config.storage.region);
    }

    #[test]
    fn test_config_not_found() {
        let temp = TempDir::new().unwrap();
        let result = LfsConfig::load(temp.path());

        assert!(matches!(result, Err(ConfigError::NotFound(_))));
    }

    #[test]
    fn test_config_validation() {
        let mut config = LfsConfig::template();
        assert!(config.validate().is_ok());

        config.storage.bucket = String::new();
        assert!(matches!(config.validate(), Err(ConfigError::Invalid(_))));
    }

    #[test]
    fn test_config_parse() {
        let toml_content = r#"
[storage]
provider = "s3"
bucket = "test-bucket"
region = "eu-west-1"
prefix = "myproject/"
"#;

        let config: LfsConfig = toml::from_str(toml_content).unwrap();
        assert_eq!(config.storage.bucket, "test-bucket");
        assert_eq!(config.storage.region, "eu-west-1");
        assert_eq!(config.storage.prefix, Some("myproject/".to_string()));
    }
}
