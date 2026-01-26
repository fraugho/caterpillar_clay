use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

use super::{StorageBackend, StorageError};

pub struct LocalStorage {
    upload_dir: PathBuf,
    base_url: String,
}

impl LocalStorage {
    pub fn new(upload_dir: &str, base_url: &str) -> Self {
        Self {
            upload_dir: PathBuf::from(upload_dir),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn ensure_dir(&self) -> Result<(), StorageError> {
        fs::create_dir_all(&self.upload_dir).await?;
        Ok(())
    }

    async fn ensure_subdir(&self, subdir: &str) -> Result<PathBuf, StorageError> {
        let dir = self.upload_dir.join(subdir);
        fs::create_dir_all(&dir).await?;
        Ok(dir)
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn upload(&self, filename: &str, data: &[u8]) -> Result<String, StorageError> {
        self.ensure_dir().await?;

        let extension = filename
            .rsplit('.')
            .next()
            .unwrap_or("bin");

        let unique_name = format!("{}.{}", Uuid::new_v4(), extension);
        let file_path = self.upload_dir.join(&unique_name);

        fs::write(&file_path, data).await?;

        Ok(format!("/uploads/{}", unique_name))
    }

    async fn upload_to_folder(&self, folder: &str, filename: &str, data: &[u8]) -> Result<String, StorageError> {
        let dir = self.ensure_subdir(folder).await?;

        let extension = filename
            .rsplit('.')
            .next()
            .unwrap_or("bin");

        let unique_name = format!("{}.{}", Uuid::new_v4(), extension);
        let file_path = dir.join(&unique_name);

        fs::write(&file_path, data).await?;

        Ok(format!("/uploads/{}/{}", folder, unique_name))
    }

    async fn delete(&self, path: &str) -> Result<(), StorageError> {
        let filename = path.trim_start_matches("/uploads/");
        let file_path = self.upload_dir.join(filename);

        if file_path.exists() {
            fs::remove_file(&file_path).await?;
        }

        Ok(())
    }

    async fn delete_folder(&self, folder: &str) -> Result<(), StorageError> {
        let dir = self.upload_dir.join(folder);
        if dir.exists() {
            fs::remove_dir_all(&dir).await?;
        }
        Ok(())
    }

    fn public_url(&self, path: &str) -> String {
        if path.starts_with("http") {
            path.to_string()
        } else {
            format!("{}{}", self.base_url, path)
        }
    }

    async fn get_object(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        let filename = path.trim_start_matches("/uploads/");
        let file_path = self.upload_dir.join(filename);
        fs::read(&file_path).await.map_err(StorageError::from)
    }

    async fn move_object(&self, from_path: &str, to_folder: &str) -> Result<String, StorageError> {
        let from_filename = from_path.trim_start_matches("/uploads/");
        let filename = from_filename
            .rsplit('/')
            .next()
            .ok_or_else(|| StorageError::UploadFailed("Invalid path".to_string()))?;

        let new_relative_path = format!("{}/{}", to_folder, filename);
        let new_full_path = format!("/uploads/{}", new_relative_path);

        // If source and destination are same, just return
        if format!("/uploads/{}", from_filename) == new_full_path {
            return Ok(new_full_path);
        }

        let from_file = self.upload_dir.join(from_filename);
        let to_dir = self.ensure_subdir(to_folder).await?;
        let to_file = to_dir.join(filename);

        fs::rename(&from_file, &to_file).await?;

        Ok(new_full_path)
    }
}
