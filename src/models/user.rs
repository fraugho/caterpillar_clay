use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub clerk_id: String,
    pub email: String,
    pub name: Option<String>,
    pub is_admin: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl User {
    pub fn uuid(&self) -> Option<Uuid> {
        Uuid::parse_str(&self.id).ok()
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub clerk_id: String,
    pub email: String,
    pub name: Option<String>,
}

impl User {
    pub async fn find_by_clerk_id(pool: &SqlitePool, clerk_id: &str) -> AppResult<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM users WHERE clerk_id = ?")
            .bind(clerk_id)
            .fetch_optional(pool)
            .await
            .map_err(AppError::from)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> AppResult<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(AppError::from)
    }

    pub async fn create(pool: &SqlitePool, data: CreateUser) -> AppResult<Self> {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO users (id, clerk_id, email, name)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&data.clerk_id)
        .bind(&data.email)
        .bind(&data.name)
        .execute(pool)
        .await
        .map_err(AppError::from)?;

        Self::find_by_id(pool, &id)
            .await?
            .ok_or_else(|| AppError::Internal("Failed to create user".to_string()))
    }

    pub async fn upsert(pool: &SqlitePool, data: CreateUser) -> AppResult<Self> {
        if let Some(existing) = Self::find_by_clerk_id(pool, &data.clerk_id).await? {
            sqlx::query(
                r#"
                UPDATE users SET email = ?, name = ?, updated_at = datetime('now')
                WHERE clerk_id = ?
                "#,
            )
            .bind(&data.email)
            .bind(&data.name)
            .bind(&data.clerk_id)
            .execute(pool)
            .await
            .map_err(AppError::from)?;

            Self::find_by_id(pool, &existing.id)
                .await?
                .ok_or_else(|| AppError::Internal("Failed to update user".to_string()))
        } else {
            Self::create(pool, data).await
        }
    }

    pub async fn set_admin(pool: &SqlitePool, id: &str, is_admin: bool) -> AppResult<Self> {
        sqlx::query(
            r#"
            UPDATE users SET is_admin = ?, updated_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(is_admin)
        .bind(id)
        .execute(pool)
        .await
        .map_err(AppError::from)?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))
    }
}
