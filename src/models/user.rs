use libsql::Connection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    fn from_row(row: &libsql::Row) -> Result<Self, libsql::Error> {
        Ok(Self {
            id: row.get(0)?,
            clerk_id: row.get(1)?,
            email: row.get(2)?,
            name: row.get(3)?,
            is_admin: row.get::<i32>(4)? != 0,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub clerk_id: String,
    pub email: String,
    pub name: Option<String>,
}

impl User {
    pub async fn find_by_clerk_id(conn: &Connection, clerk_id: &str) -> AppResult<Option<Self>> {
        let mut rows = conn
            .query("SELECT * FROM users WHERE clerk_id = ?", [clerk_id])
            .await
            .map_err(AppError::from)?;

        match rows.next().await.map_err(AppError::from)? {
            Some(row) => Ok(Some(Self::from_row(&row).map_err(AppError::from)?)),
            None => Ok(None),
        }
    }

    pub async fn find_by_id(conn: &Connection, id: &str) -> AppResult<Option<Self>> {
        let mut rows = conn
            .query("SELECT * FROM users WHERE id = ?", [id])
            .await
            .map_err(AppError::from)?;

        match rows.next().await.map_err(AppError::from)? {
            Some(row) => Ok(Some(Self::from_row(&row).map_err(AppError::from)?)),
            None => Ok(None),
        }
    }

    pub async fn create(conn: &Connection, data: CreateUser) -> AppResult<Self> {
        let id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO users (id, clerk_id, email, name) VALUES (?, ?, ?, ?)",
            libsql::params![id.clone(), data.clerk_id.clone(), data.email.clone(), data.name.clone()],
        )
        .await
        .map_err(AppError::from)?;

        Self::find_by_id(conn, &id)
            .await?
            .ok_or_else(|| AppError::Internal("Failed to create user".to_string()))
    }

    pub async fn upsert(conn: &Connection, data: CreateUser) -> AppResult<Self> {
        if let Some(existing) = Self::find_by_clerk_id(conn, &data.clerk_id).await? {
            conn.execute(
                "UPDATE users SET email = ?, name = ?, updated_at = datetime('now') WHERE clerk_id = ?",
                libsql::params![data.email.clone(), data.name.clone(), data.clerk_id.clone()],
            )
            .await
            .map_err(AppError::from)?;

            Self::find_by_id(conn, &existing.id)
                .await?
                .ok_or_else(|| AppError::Internal("Failed to update user".to_string()))
        } else {
            Self::create(conn, data).await
        }
    }

    pub async fn set_admin(conn: &Connection, id: &str, is_admin: bool) -> AppResult<Self> {
        conn.execute(
            "UPDATE users SET is_admin = ?, updated_at = datetime('now') WHERE id = ?",
            libsql::params![is_admin as i32, id.to_string()],
        )
        .await
        .map_err(AppError::from)?;

        Self::find_by_id(conn, id)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))
    }
}
