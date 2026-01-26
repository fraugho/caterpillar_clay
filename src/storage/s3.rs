use async_trait::async_trait;

use super::{StorageBackend, StorageError};

pub struct S3Storage {
    _bucket: String,
    _region: String,
}

impl S3Storage {
    pub fn new(_bucket: &str, _region: &str) -> Self {
        Self {
            _bucket: _bucket.to_string(),
            _region: _region.to_string(),
        }
    }
}

#[async_trait]
impl StorageBackend for S3Storage {
    async fn upload(&self, _filename: &str, _data: &[u8]) -> Result<String, StorageError> {
        Err(StorageError::NotConfigured(
            "S3 storage not yet implemented".to_string(),
        ))
    }

    async fn upload_to_folder(&self, _folder: &str, _filename: &str, _data: &[u8]) -> Result<String, StorageError> {
        Err(StorageError::NotConfigured(
            "S3 storage not yet implemented".to_string(),
        ))
    }

    async fn delete(&self, _path: &str) -> Result<(), StorageError> {
        Err(StorageError::NotConfigured(
            "S3 storage not yet implemented".to_string(),
        ))
    }

    async fn delete_folder(&self, _folder: &str) -> Result<(), StorageError> {
        Err(StorageError::NotConfigured(
            "S3 storage not yet implemented".to_string(),
        ))
    }

    fn public_url(&self, path: &str) -> String {
        path.to_string()
    }

    async fn get_object(&self, _path: &str) -> Result<Vec<u8>, StorageError> {
        Err(StorageError::NotConfigured(
            "S3 storage not yet implemented".to_string(),
        ))
    }

    async fn move_object(&self, _from_path: &str, _to_folder: &str) -> Result<String, StorageError> {
        Err(StorageError::NotConfigured(
            "S3 storage not yet implemented".to_string(),
        ))
    }
}
