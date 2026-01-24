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

    async fn delete(&self, path: &str) -> Result<(), StorageError> {
        let filename = path.trim_start_matches("/uploads/");
        let file_path = self.upload_dir.join(filename);

        if file_path.exists() {
            fs::remove_file(&file_path).await?;
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
}
