use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub price_cents: i32,
    pub image_path: Option<String>,
    pub stock_quantity: i32,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl Product {
    pub fn uuid(&self) -> Option<Uuid> {
        Uuid::parse_str(&self.id).ok()
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
}

impl Product {
    pub async fn list_active(pool: &SqlitePool) -> AppResult<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM products WHERE is_active = 1 ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn list_all(pool: &SqlitePool) -> AppResult<Vec<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM products ORDER BY created_at DESC")
            .fetch_all(pool)
            .await
            .map_err(AppError::from)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> AppResult<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM products WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(AppError::from)
    }

    pub async fn create(pool: &SqlitePool, data: CreateProduct) -> AppResult<Self> {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO products (id, name, description, price_cents, stock_quantity)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(data.price_cents)
        .bind(data.stock_quantity.unwrap_or(0))
        .execute(pool)
        .await
        .map_err(AppError::from)?;

        Self::find_by_id(pool, &id)
            .await?
            .ok_or_else(|| AppError::Internal("Failed to create product".to_string()))
    }

    pub async fn update(pool: &SqlitePool, id: &str, data: UpdateProduct) -> AppResult<Self> {
        let current = Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

        sqlx::query(
            r#"
            UPDATE products SET
                name = ?,
                description = ?,
                price_cents = ?,
                image_path = ?,
                stock_quantity = ?,
                is_active = ?,
                updated_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(data.name.unwrap_or(current.name))
        .bind(data.description.or(current.description))
        .bind(data.price_cents.unwrap_or(current.price_cents))
        .bind(data.image_path.or(current.image_path))
        .bind(data.stock_quantity.unwrap_or(current.stock_quantity))
        .bind(data.is_active.unwrap_or(current.is_active))
        .bind(id)
        .execute(pool)
        .await
        .map_err(AppError::from)?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Product not found".to_string()))
    }

    pub async fn set_image(pool: &SqlitePool, id: &str, image_path: &str) -> AppResult<Self> {
        sqlx::query(
            r#"
            UPDATE products SET image_path = ?, updated_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(image_path)
        .bind(id)
        .execute(pool)
        .await
        .map_err(AppError::from)?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Product not found".to_string()))
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM products WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await
            .map_err(AppError::from)?;
        Ok(())
    }

    pub async fn decrement_stock(pool: &SqlitePool, id: &str, quantity: i32) -> AppResult<Self> {
        sqlx::query(
            r#"
            UPDATE products SET
                stock_quantity = stock_quantity - ?,
                updated_at = datetime('now')
            WHERE id = ? AND stock_quantity >= ?
            "#,
        )
        .bind(quantity)
        .bind(id)
        .bind(quantity)
        .execute(pool)
        .await
        .map_err(AppError::from)?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Product not found".to_string()))
    }
}
