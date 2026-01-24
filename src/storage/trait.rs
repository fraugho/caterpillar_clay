use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Upload failed: {0}")]
    UploadFailed(String),

    #[error("Delete failed: {0}")]
    DeleteFailed(String),

    #[error("Not configured: {0}")]
    NotConfigured(String),
}

#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn upload(&self, filename: &str, data: &[u8]) -> Result<String, StorageError>;
    async fn delete(&self, path: &str) -> Result<(), StorageError>;
    fn public_url(&self, path: &str) -> String;
}
