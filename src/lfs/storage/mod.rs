//! Storage backends for LFS
//!
//! Provides a trait for storage operations and implementations for different providers.

pub mod s3;

use async_trait::async_trait;
use std::path::Path;
use thiserror::Error;

pub use s3::{S3Config, S3Storage};

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum StorageError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Object not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("AWS SDK error: {0}")]
    AwsSdk(String),
}

/// Result of an upload operation
#[derive(Debug)]
#[allow(dead_code)]
pub struct UploadResult {
    /// The object ID (sha256 hash)
    pub oid: String,
    /// Size in bytes
    pub size: u64,
    /// Whether the object was newly uploaded (false if already existed)
    pub uploaded: bool,
}

/// Result of a download operation
#[derive(Debug)]
#[allow(dead_code)]
pub struct DownloadResult {
    /// The object ID
    pub oid: String,
    /// Size in bytes
    pub size: u64,
    /// Path where the file was downloaded
    pub path: std::path::PathBuf,
}

/// Trait for LFS storage backends
#[async_trait]
#[allow(dead_code)]
pub trait Storage: Send + Sync {
    /// Upload a file to storage
    async fn upload(&self, oid: &str, source: &Path) -> Result<UploadResult, StorageError>;

    /// Download a file from storage
    async fn download(&self, oid: &str, dest: &Path) -> Result<DownloadResult, StorageError>;

    /// Check if an object exists in storage
    async fn exists(&self, oid: &str) -> Result<bool, StorageError>;

    /// Delete an object from storage
    async fn delete(&self, oid: &str) -> Result<(), StorageError>;

    /// Get the storage provider name
    fn provider_name(&self) -> &str;
}
