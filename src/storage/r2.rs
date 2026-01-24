use async_trait::async_trait;
use s3::creds::Credentials;
use s3::{Bucket, Region};
use uuid::Uuid;

use super::{StorageBackend, StorageError};

pub struct R2Storage {
    bucket: Box<Bucket>,
    public_url: String,
}

impl R2Storage {
    pub fn new(
        bucket_name: &str,
        account_id: &str,
        access_key: &str,
        secret_key: &str,
        public_url: &str,
    ) -> Result<Self, StorageError> {
        let endpoint = format!("https://{}.r2.cloudflarestorage.com", account_id);

        let region = Region::Custom {
            region: "auto".to_string(),
            endpoint,
        };

        let credentials = Credentials::new(
            Some(access_key),
            Some(secret_key),
            None,
            None,
            None,
        )
        .map_err(|e| StorageError::NotConfigured(e.to_string()))?;

        let bucket = Bucket::new(bucket_name, region, credentials)
            .map_err(|e| StorageError::NotConfigured(e.to_string()))?
            .with_path_style();

        Ok(Self {
            bucket,
            public_url: public_url.trim_end_matches('/').to_string(),
        })
    }
}

#[async_trait]
impl StorageBackend for R2Storage {
    async fn upload(&self, filename: &str, data: &[u8]) -> Result<String, StorageError> {
        let extension = filename.rsplit('.').next().unwrap_or("bin");
        let unique_name = format!("{}.{}", Uuid::new_v4(), extension);
        let path = format!("uploads/{}", unique_name);

        let content_type = match extension.to_lowercase().as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            _ => "application/octet-stream",
        };

        self.bucket
            .put_object_with_content_type(&path, data, content_type)
            .await
            .map_err(|e| StorageError::UploadFailed(e.to_string()))?;

        Ok(format!("/{}", path))
    }

    async fn delete(&self, path: &str) -> Result<(), StorageError> {
        let path = path.trim_start_matches('/');

        self.bucket
            .delete_object(path)
            .await
            .map_err(|e| StorageError::DeleteFailed(e.to_string()))?;

        Ok(())
    }

    fn public_url(&self, path: &str) -> String {
        if path.starts_with("http") {
            path.to_string()
        } else {
            format!("{}{}", self.public_url, path)
        }
    }
}
