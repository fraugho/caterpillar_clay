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

    pub async fn get_shop_address(conn: &Connection) -> AppResult<Option<ShopAddress>> {
        let street1 = Self::get(conn, "shop_street1").await?.unwrap_or_default();
        if street1.is_empty() {
            return Ok(None);
        }

        Ok(Some(ShopAddress {
            name: Self::get(conn, "shop_name").await?.unwrap_or_else(|| "Caterpillar Clay".to_string()),
            street1,
            street2: Self::get(conn, "shop_street2").await?.filter(|s| !s.is_empty()),
            city: Self::get(conn, "shop_city").await?.unwrap_or_default(),
            state: Self::get(conn, "shop_state").await?.unwrap_or_default(),
            zip: Self::get(conn, "shop_zip").await?.unwrap_or_default(),
            country: Self::get(conn, "shop_country").await?.unwrap_or_else(|| "US".to_string()),
            phone: Self::get(conn, "shop_phone").await?.filter(|s| !s.is_empty()),
        }))
    }

    pub async fn get_unit_system(conn: &Connection) -> AppResult<String> {
        Ok(Self::get(conn, "shipping_unit_system").await?.unwrap_or_else(|| "metric".to_string()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistInfo {
    pub image: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShopAddress {
    pub name: String,
    pub street1: String,
    pub street2: Option<String>,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub country: String,
    pub phone: Option<String>,
}
