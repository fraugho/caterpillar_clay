use libsql::Connection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductNotification {
    pub id: String,
    pub email: String,
    pub product_id: String,
    pub notified: bool,
    pub created_ts: i64,
    pub notified_ts: Option<i64>,
}

impl ProductNotification {
    /// Subscribe an email to be notified when a product is back in stock
    pub async fn subscribe(conn: &Connection, email: &str, product_id: &str) -> AppResult<Self> {
        let email = email.to_lowercase();

        // Check if already subscribed (and not yet notified)
        if let Some(existing) = Self::find_pending(conn, &email, product_id).await? {
            return Ok(existing);
        }

        let id = Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO product_notifications (id, email, product_id, notified, created_ts) VALUES (?, ?, ?, 0, ?)",
            libsql::params![id.clone(), email.clone(), product_id, now],
        )
        .await
        .map_err(AppError::from)?;

        Ok(Self {
            id,
            email,
            product_id: product_id.to_string(),
            notified: false,
            created_ts: now,
            notified_ts: None,
        })
    }

    /// Find a pending (not yet notified) notification for an email and product
    pub async fn find_pending(conn: &Connection, email: &str, product_id: &str) -> AppResult<Option<Self>> {
        let mut rows = conn
            .query(
                "SELECT id, email, product_id, notified, created_ts, notified_ts
                 FROM product_notifications
                 WHERE email = ? AND product_id = ? AND notified = 0",
                libsql::params![email.to_lowercase(), product_id],
            )
            .await
            .map_err(AppError::from)?;

        if let Some(row) = rows.next().await.map_err(AppError::from)? {
            Ok(Some(Self::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    /// Get all pending notifications for a product (for sending restock emails)
    pub async fn get_pending_for_product(conn: &Connection, product_id: &str) -> AppResult<Vec<Self>> {
        let mut rows = conn
            .query(
                "SELECT id, email, product_id, notified, created_ts, notified_ts
                 FROM product_notifications
                 WHERE product_id = ? AND notified = 0
                 ORDER BY created_ts ASC",
                [product_id],
            )
            .await
            .map_err(AppError::from)?;

        let mut notifications = Vec::new();
        while let Some(row) = rows.next().await.map_err(AppError::from)? {
            notifications.push(Self::from_row(&row)?);
        }

        Ok(notifications)
    }

    /// Mark a notification as sent
    pub async fn mark_notified(conn: &Connection, id: &str) -> AppResult<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "UPDATE product_notifications SET notified = 1, notified_ts = ? WHERE id = ?",
            libsql::params![now, id],
        )
        .await
        .map_err(AppError::from)?;

        Ok(())
    }

    /// Mark all pending notifications for a product as notified
    pub async fn mark_all_notified_for_product(conn: &Connection, product_id: &str) -> AppResult<u64> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let result = conn
            .execute(
                "UPDATE product_notifications SET notified = 1, notified_ts = ? WHERE product_id = ? AND notified = 0",
                libsql::params![now, product_id],
            )
            .await
            .map_err(AppError::from)?;

        Ok(result)
    }

    /// Count pending notifications for a product
    pub async fn count_pending_for_product(conn: &Connection, product_id: &str) -> AppResult<i64> {
        let mut rows = conn
            .query(
                "SELECT COUNT(*) FROM product_notifications WHERE product_id = ? AND notified = 0",
                [product_id],
            )
            .await
            .map_err(AppError::from)?;

        if let Some(row) = rows.next().await.map_err(AppError::from)? {
            let count: i64 = row.get(0).map_err(AppError::from)?;
            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Clean up old notified entries (optional maintenance)
    pub async fn cleanup_old_notified(conn: &Connection, older_than_days: i64) -> AppResult<u64> {
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - (older_than_days * 24 * 60 * 60);

        let result = conn
            .execute(
                "DELETE FROM product_notifications WHERE notified = 1 AND notified_ts < ?",
                [cutoff],
            )
            .await
            .map_err(AppError::from)?;

        Ok(result)
    }

    fn from_row(row: &libsql::Row) -> AppResult<Self> {
        Ok(Self {
            id: row.get(0).map_err(AppError::from)?,
            email: row.get(1).map_err(AppError::from)?,
            product_id: row.get(2).map_err(AppError::from)?,
            notified: row.get::<i64>(3).map_err(AppError::from)? != 0,
            created_ts: row.get(4).map_err(AppError::from)?,
            notified_ts: row.get(5).map_err(AppError::from).ok(),
        })
    }
}
