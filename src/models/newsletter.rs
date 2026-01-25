use libsql::Connection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsletterSubscriber {
    pub id: String,
    pub email: String,
    pub subscribed_ts: i64,
    pub unsubscribe_token: String,
}

impl NewsletterSubscriber {
    pub async fn subscribe(conn: &Connection, email: &str) -> AppResult<Self> {
        // Check if already subscribed
        if let Some(existing) = Self::find_by_email(conn, email).await? {
            return Ok(existing);
        }

        let id = Uuid::new_v4().to_string();
        let unsubscribe_token = Uuid::new_v4().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO newsletter_subscribers (id, email, subscribed_ts, unsubscribe_token) VALUES (?, ?, ?, ?)",
            libsql::params![id.clone(), email.to_lowercase(), now, unsubscribe_token.clone()],
        )
        .await
        .map_err(AppError::from)?;

        Ok(Self {
            id,
            email: email.to_lowercase(),
            subscribed_ts: now,
            unsubscribe_token,
        })
    }

    pub async fn unsubscribe_by_token(conn: &Connection, token: &str) -> AppResult<bool> {
        let result = conn
            .execute(
                "DELETE FROM newsletter_subscribers WHERE unsubscribe_token = ?",
                [token],
            )
            .await
            .map_err(AppError::from)?;

        Ok(result > 0)
    }

    pub async fn unsubscribe_by_email(conn: &Connection, email: &str) -> AppResult<bool> {
        let result = conn
            .execute(
                "DELETE FROM newsletter_subscribers WHERE email = ?",
                [email.to_lowercase()],
            )
            .await
            .map_err(AppError::from)?;

        Ok(result > 0)
    }

    pub async fn find_by_email(conn: &Connection, email: &str) -> AppResult<Option<Self>> {
        let mut rows = conn
            .query(
                "SELECT id, email, subscribed_ts, unsubscribe_token FROM newsletter_subscribers WHERE email = ?",
                [email.to_lowercase()],
            )
            .await
            .map_err(AppError::from)?;

        if let Some(row) = rows.next().await.map_err(AppError::from)? {
            Ok(Some(Self {
                id: row.get(0).map_err(AppError::from)?,
                email: row.get(1).map_err(AppError::from)?,
                subscribed_ts: row.get(2).map_err(AppError::from)?,
                unsubscribe_token: row.get(3).map_err(AppError::from)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get_all(conn: &Connection) -> AppResult<Vec<Self>> {
        let mut rows = conn
            .query(
                "SELECT id, email, subscribed_ts, unsubscribe_token FROM newsletter_subscribers ORDER BY subscribed_ts DESC",
                (),
            )
            .await
            .map_err(AppError::from)?;

        let mut subscribers = Vec::new();
        while let Some(row) = rows.next().await.map_err(AppError::from)? {
            subscribers.push(Self {
                id: row.get(0).map_err(AppError::from)?,
                email: row.get(1).map_err(AppError::from)?,
                subscribed_ts: row.get(2).map_err(AppError::from)?,
                unsubscribe_token: row.get(3).map_err(AppError::from)?,
            });
        }

        Ok(subscribers)
    }

    pub async fn count(conn: &Connection) -> AppResult<i64> {
        let mut rows = conn
            .query("SELECT COUNT(*) FROM newsletter_subscribers", ())
            .await
            .map_err(AppError::from)?;

        if let Some(row) = rows.next().await.map_err(AppError::from)? {
            let count: i64 = row.get(0).map_err(AppError::from)?;
            Ok(count)
        } else {
            Ok(0)
        }
    }
}
