use redis::{AsyncCommands, Client};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Distributed rate limiter using Upstash Redis
#[derive(Clone)]
pub struct RateLimiter {
    client: Client,
    connection: Arc<Mutex<Option<redis::aio::MultiplexedConnection>>>,
    requests_per_minute: u32,
}

impl RateLimiter {
    pub fn new(redis_url: &str, requests_per_minute: u32) -> Result<Self, RateLimitError> {
        let client = Client::open(redis_url)
            .map_err(|e| RateLimitError::Connection(e.to_string()))?;

        Ok(Self {
            client,
            connection: Arc::new(Mutex::new(None)),
            requests_per_minute,
        })
    }

    async fn get_connection(&self) -> Result<redis::aio::MultiplexedConnection, RateLimitError> {
        let mut conn_guard = self.connection.lock().await;

        if let Some(ref conn) = *conn_guard {
            return Ok(conn.clone());
        }

        let conn = self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| RateLimitError::Connection(e.to_string()))?;

        *conn_guard = Some(conn.clone());
        Ok(conn)
    }

    /// Check if request is allowed for the given IP
    /// Returns Ok(true) if allowed, Ok(false) if rate limited
    pub async fn check_rate_limit(&self, ip: &str) -> Result<bool, RateLimitError> {
        let key = format!("rate_limit:{}", ip);
        let window_seconds: i64 = 60;

        let mut conn = self.get_connection().await?;

        // Increment the counter
        let count: i64 = conn.incr(&key, 1)
            .await
            .map_err(|e| RateLimitError::Redis(e.to_string()))?;

        // If this is the first request in the window, set expiry
        if count == 1 {
            let _: () = conn.expire(&key, window_seconds)
                .await
                .map_err(|e| RateLimitError::Redis(e.to_string()))?;
        }

        Ok(count <= self.requests_per_minute as i64)
    }

    /// Get remaining requests for the given IP
    pub async fn get_remaining(&self, ip: &str) -> Result<u32, RateLimitError> {
        let key = format!("rate_limit:{}", ip);

        let mut conn = self.get_connection().await?;

        let count: Option<i64> = conn.get(&key)
            .await
            .map_err(|e| RateLimitError::Redis(e.to_string()))?;

        let count = count.unwrap_or(0) as u32;
        Ok(self.requests_per_minute.saturating_sub(count))
    }
}

#[derive(Debug)]
pub enum RateLimitError {
    Connection(String),
    Redis(String),
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateLimitError::Connection(e) => write!(f, "Redis connection error: {}", e),
            RateLimitError::Redis(e) => write!(f, "Redis error: {}", e),
        }
    }
}
