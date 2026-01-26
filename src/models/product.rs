use libsql::Connection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductImage {
    pub id: String,
    pub product_id: String,
    pub image_path: String,
    pub sort_order: i32,
    pub created_ts: i64,
}

impl ProductImage {
    fn from_row(row: &libsql::Row) -> Result<Self, libsql::Error> {
        Ok(Self {
            id: row.get(0)?,
            product_id: row.get(1)?,
            image_path: row.get(2)?,
            sort_order: row.get(3)?,
            created_ts: row.get(4)?,
        })
    }

    pub async fn list_by_product(conn: &Connection, product_id: &str) -> AppResult<Vec<Self>> {
        let mut rows = conn
            .query(
                "SELECT * FROM product_images WHERE product_id = ? ORDER BY sort_order ASC",
                [product_id],
            )
            .await
            .map_err(AppError::from)?;

        let mut images = Vec::new();
        while let Some(row) = rows.next().await.map_err(AppError::from)? {
            images.push(Self::from_row(&row).map_err(AppError::from)?);
        }
        Ok(images)
    }

    pub async fn add(conn: &Connection, product_id: &str, image_path: &str) -> AppResult<Self> {
        let id = Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Get next sort order
        let mut rows = conn
            .query(
                "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM product_images WHERE product_id = ?",
                [product_id],
            )
            .await
            .map_err(AppError::from)?;

        let sort_order: i32 = match rows.next().await.map_err(AppError::from)? {
            Some(row) => row.get(0).unwrap_or(0),
            None => 0,
        };

        conn.execute(
            "INSERT INTO product_images (id, product_id, image_path, sort_order, created_ts) VALUES (?, ?, ?, ?, ?)",
            libsql::params![id.clone(), product_id.to_string(), image_path.to_string(), sort_order, now],
        )
        .await
        .map_err(AppError::from)?;

        Ok(Self {
            id,
            product_id: product_id.to_string(),
            image_path: image_path.to_string(),
            sort_order,
            created_ts: now,
        })
    }

    pub async fn delete(conn: &Connection, id: &str) -> AppResult<()> {
        conn.execute("DELETE FROM product_images WHERE id = ?", [id.to_string()])
            .await
            .map_err(AppError::from)?;
        Ok(())
    }

    pub async fn delete_by_product(conn: &Connection, product_id: &str) -> AppResult<()> {
        conn.execute(
            "DELETE FROM product_images WHERE product_id = ?",
            [product_id.to_string()],
        )
        .await
        .map_err(AppError::from)?;
        Ok(())
    }

    pub async fn reorder(conn: &Connection, product_id: &str, image_ids: &[String]) -> AppResult<()> {
        for (idx, image_id) in image_ids.iter().enumerate() {
            conn.execute(
                "UPDATE product_images SET sort_order = ? WHERE id = ? AND product_id = ?",
                libsql::params![idx as i32, image_id.to_string(), product_id.to_string()],
            )
            .await
            .map_err(AppError::from)?;
        }
        Ok(())
    }

    pub async fn find_by_id(conn: &Connection, id: &str) -> AppResult<Option<Self>> {
        let mut rows = conn
            .query("SELECT * FROM product_images WHERE id = ?", [id])
            .await
            .map_err(AppError::from)?;

        match rows.next().await.map_err(AppError::from)? {
            Some(row) => Ok(Some(Self::from_row(&row).map_err(AppError::from)?)),
            None => Ok(None),
        }
    }

    pub async fn update_path(conn: &Connection, id: &str, new_path: &str) -> AppResult<()> {
        conn.execute(
            "UPDATE product_images SET image_path = ? WHERE id = ?",
            libsql::params![new_path.to_string(), id.to_string()],
        )
        .await
        .map_err(AppError::from)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub price_cents: i32,
    pub image_path: Option<String>,
    pub stock_quantity: i32,
    pub is_active: bool,
    pub polar_price_id: Option<String>,
    pub polar_product_id: Option<String>,
    pub created_ts: i64,
    pub updated_ts: i64,
}

impl Product {
    pub fn uuid(&self) -> Option<Uuid> {
        Uuid::parse_str(&self.id).ok()
    }

    fn from_row(row: &libsql::Row) -> Result<Self, libsql::Error> {
        Ok(Self {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            price_cents: row.get(3)?,
            image_path: row.get(4)?,
            stock_quantity: row.get(5)?,
            is_active: row.get::<i32>(6)? != 0,
            polar_price_id: row.get(9)?,
            polar_product_id: row.get(10)?,
            created_ts: row.get(11)?,
            updated_ts: row.get(12)?,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateProduct {
    pub name: String,
    pub description: Option<String>,
    pub price_cents: i32,
    pub stock_quantity: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProduct {
    pub name: Option<String>,
    pub description: Option<String>,
    pub price_cents: Option<i32>,
    pub image_path: Option<String>,
    pub stock_quantity: Option<i32>,
    pub is_active: Option<bool>,
    pub polar_price_id: Option<String>,
}

impl Product {
    pub async fn list_active(conn: &Connection) -> AppResult<Vec<Self>> {
        let mut rows = conn
            .query(
                "SELECT * FROM products WHERE is_active = 1 ORDER BY created_ts DESC",
                (),
            )
            .await
            .map_err(AppError::from)?;

        let mut products = Vec::new();
        while let Some(row) = rows.next().await.map_err(AppError::from)? {
            products.push(Self::from_row(&row).map_err(AppError::from)?);
        }
        Ok(products)
    }

    pub async fn list_all(conn: &Connection) -> AppResult<Vec<Self>> {
        let mut rows = conn
            .query("SELECT * FROM products ORDER BY created_ts DESC", ())
            .await
            .map_err(AppError::from)?;

        let mut products = Vec::new();
        while let Some(row) = rows.next().await.map_err(AppError::from)? {
            products.push(Self::from_row(&row).map_err(AppError::from)?);
        }
        Ok(products)
    }

    pub async fn find_by_id(conn: &Connection, id: &str) -> AppResult<Option<Self>> {
        let mut rows = conn
            .query("SELECT * FROM products WHERE id = ?", [id])
            .await
            .map_err(AppError::from)?;

        match rows.next().await.map_err(AppError::from)? {
            Some(row) => Ok(Some(Self::from_row(&row).map_err(AppError::from)?)),
            None => Ok(None),
        }
    }

    pub async fn create(conn: &Connection, data: CreateProduct) -> AppResult<Self> {
        let id = Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO products (id, name, description, price_cents, stock_quantity, created_ts, updated_ts) VALUES (?, ?, ?, ?, ?, ?, ?)",
            libsql::params![id.clone(), data.name, data.description, data.price_cents, data.stock_quantity.unwrap_or(0), now, now],
        )
        .await
        .map_err(AppError::from)?;

        Self::find_by_id(conn, &id)
            .await?
            .ok_or_else(|| AppError::Internal("Failed to create product".to_string()))
    }

    pub async fn update(conn: &Connection, id: &str, data: UpdateProduct) -> AppResult<Self> {
        let current = Self::find_by_id(conn, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

        let name = data.name.unwrap_or(current.name);
        let description = data.description.or(current.description);
        let price_cents = data.price_cents.unwrap_or(current.price_cents);
        let image_path = data.image_path.or(current.image_path);
        let stock_quantity = data.stock_quantity.unwrap_or(current.stock_quantity);
        let is_active = data.is_active.unwrap_or(current.is_active) as i32;
        let polar_price_id = data.polar_price_id.or(current.polar_price_id);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            r#"
            UPDATE products SET
                name = ?,
                description = ?,
                price_cents = ?,
                image_path = ?,
                stock_quantity = ?,
                is_active = ?,
                polar_price_id = ?,
                updated_ts = ?
            WHERE id = ?
            "#,
            libsql::params![name, description, price_cents, image_path, stock_quantity, is_active, polar_price_id, now, id.to_string()],
        )
        .await
        .map_err(AppError::from)?;

        Self::find_by_id(conn, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Product not found".to_string()))
    }

    pub async fn set_image(conn: &Connection, id: &str, image_path: &str) -> AppResult<Self> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "UPDATE products SET image_path = ?, updated_ts = ? WHERE id = ?",
            libsql::params![image_path.to_string(), now, id.to_string()],
        )
        .await
        .map_err(AppError::from)?;

        Self::find_by_id(conn, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Product not found".to_string()))
    }

    pub async fn set_polar_ids(
        conn: &Connection,
        id: &str,
        polar_product_id: &str,
        polar_price_id: &str,
    ) -> AppResult<Self> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "UPDATE products SET polar_product_id = ?, polar_price_id = ?, updated_ts = ? WHERE id = ?",
            libsql::params![polar_product_id.to_string(), polar_price_id.to_string(), now, id.to_string()],
        )
        .await
        .map_err(AppError::from)?;

        Self::find_by_id(conn, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Product not found".to_string()))
    }

    pub async fn delete(conn: &Connection, id: &str) -> AppResult<()> {
        conn.execute("DELETE FROM products WHERE id = ?", [id.to_string()])
            .await
            .map_err(AppError::from)?;
        Ok(())
    }

    pub async fn decrement_stock(conn: &Connection, id: &str, quantity: i32) -> AppResult<Self> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "UPDATE products SET stock_quantity = stock_quantity - ?, updated_ts = ? WHERE id = ? AND stock_quantity >= ?",
            libsql::params![quantity, now, id.to_string(), quantity],
        )
        .await
        .map_err(AppError::from)?;

        Self::find_by_id(conn, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Product not found".to_string()))
    }
}
