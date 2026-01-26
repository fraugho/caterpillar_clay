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
    async fn upload_to_folder(&self, folder: &str, filename: &str, data: &[u8]) -> Result<String, StorageError>;
    async fn delete(&self, path: &str) -> Result<(), StorageError>;
    async fn delete_folder(&self, folder: &str) -> Result<(), StorageError>;
    fn public_url(&self, path: &str) -> String;
    /// Get raw object data from storage
    async fn get_object(&self, path: &str) -> Result<Vec<u8>, StorageError>;
    /// Move/rename an object to a new path, returns the new path
    async fn move_object(&self, from_path: &str, to_folder: &str) -> Result<String, StorageError>;
}
