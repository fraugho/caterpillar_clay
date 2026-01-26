use libsql::Connection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductStyle {
    pub id: String,
    pub product_id: String,
    pub name: String,
    pub stock_quantity: i64,
    pub image_id: Option<String>,
    pub sort_order: i64,
    pub created_ts: i64,
    // Populated when fetching with image info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_path: Option<String>,
}

impl ProductStyle {
    pub async fn create(
        conn: &Connection,
        product_id: &str,
        name: &str,
        stock_quantity: i64,
        image_id: Option<&str>,
    ) -> AppResult<Self> {
        let id = Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Get next sort order
        let mut rows = conn
            .query(
                "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM product_styles WHERE product_id = ?",
                [product_id],
            )
            .await
            .map_err(AppError::from)?;

        let sort_order: i64 = if let Some(row) = rows.next().await.map_err(AppError::from)? {
            row.get(0).unwrap_or(0)
        } else {
            0
        };

        conn.execute(
            "INSERT INTO product_styles (id, product_id, name, stock_quantity, image_id, sort_order, created_ts)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            libsql::params![id.clone(), product_id, name, stock_quantity, image_id, sort_order, now],
        )
        .await
        .map_err(AppError::from)?;

        Ok(Self {
            id,
            product_id: product_id.to_string(),
            name: name.to_string(),
            stock_quantity,
            image_id: image_id.map(|s| s.to_string()),
            sort_order,
            created_ts: now,
            image_path: None,
        })
    }

    pub async fn get_by_product(conn: &Connection, product_id: &str) -> AppResult<Vec<Self>> {
        let mut rows = conn
            .query(
                "SELECT ps.id, ps.product_id, ps.name, ps.stock_quantity, ps.image_id, ps.sort_order, ps.created_ts, pi.image_path
                 FROM product_styles ps
                 LEFT JOIN product_images pi ON ps.image_id = pi.id
                 WHERE ps.product_id = ?
                 ORDER BY ps.sort_order ASC",
                [product_id],
            )
            .await
            .map_err(AppError::from)?;

        let mut styles = Vec::new();
        while let Some(row) = rows.next().await.map_err(AppError::from)? {
            styles.push(Self::from_row(&row)?);
        }

        Ok(styles)
    }

    pub async fn get_by_id(conn: &Connection, id: &str) -> AppResult<Option<Self>> {
        let mut rows = conn
            .query(
                "SELECT ps.id, ps.product_id, ps.name, ps.stock_quantity, ps.image_id, ps.sort_order, ps.created_ts, pi.image_path
                 FROM product_styles ps
                 LEFT JOIN product_images pi ON ps.image_id = pi.id
                 WHERE ps.id = ?",
                [id],
            )
            .await
            .map_err(AppError::from)?;

        if let Some(row) = rows.next().await.map_err(AppError::from)? {
            Ok(Some(Self::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    pub async fn update(
        conn: &Connection,
        id: &str,
        name: &str,
        stock_quantity: i64,
        image_id: Option<&str>,
    ) -> AppResult<()> {
        conn.execute(
            "UPDATE product_styles SET name = ?, stock_quantity = ?, image_id = ? WHERE id = ?",
            libsql::params![name, stock_quantity, image_id, id],
        )
        .await
        .map_err(AppError::from)?;

        Ok(())
    }

    pub async fn update_stock(conn: &Connection, id: &str, stock_quantity: i64) -> AppResult<()> {
        conn.execute(
            "UPDATE product_styles SET stock_quantity = ? WHERE id = ?",
            libsql::params![stock_quantity, id],
        )
        .await
        .map_err(AppError::from)?;

        Ok(())
    }

    pub async fn delete(conn: &Connection, id: &str) -> AppResult<()> {
        conn.execute("DELETE FROM product_styles WHERE id = ?", [id])
            .await
            .map_err(AppError::from)?;

        Ok(())
    }

    pub async fn reorder(conn: &Connection, product_id: &str, style_ids: &[String]) -> AppResult<()> {
        for (idx, style_id) in style_ids.iter().enumerate() {
            conn.execute(
                "UPDATE product_styles SET sort_order = ? WHERE id = ? AND product_id = ?",
                libsql::params![idx as i64, style_id.clone(), product_id],
            )
            .await
            .map_err(AppError::from)?;
        }

        Ok(())
    }

    /// Get styles that were out of stock but now have stock
    pub async fn get_restocked_styles(
        conn: &Connection,
        product_id: &str,
        style_ids: &[String],
    ) -> AppResult<Vec<Self>> {
        if style_ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders: Vec<&str> = style_ids.iter().map(|_| "?").collect();
        let query = format!(
            "SELECT ps.id, ps.product_id, ps.name, ps.stock_quantity, ps.image_id, ps.sort_order, ps.created_ts, pi.image_path
             FROM product_styles ps
             LEFT JOIN product_images pi ON ps.image_id = pi.id
             WHERE ps.product_id = ? AND ps.id IN ({}) AND ps.stock_quantity > 0
             ORDER BY ps.sort_order ASC",
            placeholders.join(", ")
        );

        let mut params: Vec<libsql::Value> = vec![product_id.into()];
        for id in style_ids {
            params.push(id.clone().into());
        }

        let mut rows = conn.query(&query, params).await.map_err(AppError::from)?;

        let mut styles = Vec::new();
        while let Some(row) = rows.next().await.map_err(AppError::from)? {
            styles.push(Self::from_row(&row)?);
        }

        Ok(styles)
    }

    fn from_row(row: &libsql::Row) -> AppResult<Self> {
        Ok(Self {
            id: row.get(0).map_err(AppError::from)?,
            product_id: row.get(1).map_err(AppError::from)?,
            name: row.get(2).map_err(AppError::from)?,
            stock_quantity: row.get(3).map_err(AppError::from)?,
            image_id: row.get(4).map_err(AppError::from).ok(),
            sort_order: row.get(5).map_err(AppError::from)?,
            created_ts: row.get(6).map_err(AppError::from)?,
            image_path: row.get(7).map_err(AppError::from).ok(),
        })
    }
}
