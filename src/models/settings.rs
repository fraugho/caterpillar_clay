use libsql::Connection;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
    pub updated_ts: i64,
}

impl Setting {
    pub async fn get(conn: &Connection, key: &str) -> AppResult<Option<String>> {
        let mut rows = conn
            .query("SELECT value FROM site_settings WHERE key = ?", [key])
            .await
            .map_err(AppError::from)?;

        if let Some(row) = rows.next().await.map_err(AppError::from)? {
            let value: String = row.get(0).map_err(AppError::from)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    pub async fn set(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT OR REPLACE INTO site_settings (key, value, updated_ts) VALUES (?, ?, ?)",
            libsql::params![key.to_string(), value.to_string(), now],
        )
        .await
        .map_err(AppError::from)?;

        Ok(())
    }

    pub async fn get_artist_info(conn: &Connection) -> AppResult<ArtistInfo> {
        let image = Self::get(conn, "artist_image").await?.unwrap_or_else(|| "/artist/Alex.webp".to_string());
        let description = Self::get(conn, "artist_description").await?.unwrap_or_else(|| "".to_string());

        Ok(ArtistInfo { image, description })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistInfo {
    pub image: String,
    pub description: String,
}
