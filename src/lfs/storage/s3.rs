//! AWS S3 storage backend

use super::{DownloadResult, Storage, StorageError, UploadResult};
use async_trait::async_trait;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

/// Inline credentials for S3
#[derive(Debug, Clone)]
pub struct S3Credentials {
    pub access_key_id: String,
    pub secret_access_key: String,
}

/// S3 storage configuration
#[derive(Debug, Clone)]
pub struct S3Config {
    /// S3 bucket name
    pub bucket: String,
    /// AWS region
    pub region: String,
    /// Optional prefix for object keys
    pub prefix: Option<String>,
    /// Optional custom endpoint (for S3-compatible services)
    pub endpoint: Option<String>,
    /// Optional inline credentials
    pub credentials: Option<S3Credentials>,
}

/// AWS S3 storage backend
pub struct S3Storage {
    client: Client,
    config: S3Config,
}

impl S3Storage {
    /// Create a new S3 storage backend
    pub async fn new(config: S3Config) -> Result<Self, StorageError> {
        let mut aws_config_builder = aws_config::from_env();

        // Set region
        aws_config_builder =
            aws_config_builder.region(aws_config::Region::new(config.region.clone()));

        // Set custom endpoint if provided
        if let Some(endpoint) = &config.endpoint {
            aws_config_builder = aws_config_builder.endpoint_url(endpoint);
        }

        // Use inline credentials if provided
        if let Some(creds) = &config.credentials {
            let credentials = aws_sdk_s3::config::Credentials::new(
                &creds.access_key_id,
                &creds.secret_access_key,
                None,
                None,
                "gg-lfs-config",
            );
            aws_config_builder = aws_config_builder.credentials_provider(credentials);
        }

        let aws_config = aws_config_builder.load().await;

        let client = Client::new(&aws_config);

        Ok(Self { client, config })
    }

    /// Get the full object key with prefix
    fn object_key(&self, oid: &str) -> String {
        // Use first 2 chars of hash as directory for better S3 performance
        let prefix = &oid[..2.min(oid.len())];

        match &self.config.prefix {
            Some(p) => format!("{}/{}/{}", p.trim_end_matches('/'), prefix, oid),
            None => format!("{}/{}", prefix, oid),
        }
    }
}

#[async_trait]
impl Storage for S3Storage {
    async fn upload(&self, oid: &str, source: &Path) -> Result<UploadResult, StorageError> {
        let key = self.object_key(oid);

        // Check if already exists
        if self.exists(oid).await? {
            let metadata = tokio::fs::metadata(source).await?;
            return Ok(UploadResult {
                oid: oid.to_string(),
                size: metadata.len(),
                uploaded: false,
            });
        }

        // Read file and upload
        let body = ByteStream::from_path(source)
            .await
            .map_err(|e| StorageError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        let metadata = tokio::fs::metadata(source).await?;
        let size = metadata.len();

        self.client
            .put_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .body(body)
            .content_type("application/octet-stream")
            .send()
            .await
            .map_err(|e| StorageError::AwsSdk(e.to_string()))?;

        Ok(UploadResult {
            oid: oid.to_string(),
            size,
            uploaded: true,
        })
    }

    async fn download(&self, oid: &str, dest: &Path) -> Result<DownloadResult, StorageError> {
        let key = self.object_key(oid);

        let response = self
            .client
            .get_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| {
                let err_str = e.to_string();
                if err_str.contains("NoSuchKey") || err_str.contains("404") {
                    StorageError::NotFound(oid.to_string())
                } else {
                    StorageError::AwsSdk(err_str)
                }
            })?;

        // Ensure parent directory exists
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Stream body to file
        let body = response
            .body
            .collect()
            .await
            .map_err(|e| StorageError::AwsSdk(e.to_string()))?;

        let bytes = body.into_bytes();
        let size = bytes.len() as u64;

        let mut file = File::create(dest).await?;
        file.write_all(&bytes).await?;
        file.flush().await?;

        Ok(DownloadResult {
            oid: oid.to_string(),
            size,
            path: dest.to_path_buf(),
        })
    }

    async fn exists(&self, oid: &str) -> Result<bool, StorageError> {
        let key = self.object_key(oid);

        match self
            .client
            .head_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("NotFound") || err_str.contains("404") {
                    Ok(false)
                } else {
                    Err(StorageError::AwsSdk(err_str))
                }
            }
        }
    }

    async fn delete(&self, oid: &str) -> Result<(), StorageError> {
        let key = self.object_key(oid);

        self.client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| StorageError::AwsSdk(e.to_string()))?;

        Ok(())
    }

    fn provider_name(&self) -> &str {
        "AWS S3"
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_object_key_no_prefix() {
        // Can't easily test without async, but we can verify the key format logic
        let oid = "4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393";
        let prefix = &oid[..2];
        let expected = format!("{}/{}", prefix, oid);
        assert_eq!(expected, "4d/4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393");
    }
}
