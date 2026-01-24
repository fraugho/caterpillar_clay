use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct ClerkService {
    client: Client,
    secret_key: String,
}

#[derive(Debug, Deserialize)]
pub struct ClerkUser {
    pub id: String,
    pub email_addresses: Vec<ClerkEmailAddress>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ClerkEmailAddress {
    pub email_address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClerkJwks {
    pub keys: Vec<ClerkJwk>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClerkJwk {
    pub kty: String,
    #[serde(rename = "use")]
    pub use_: String,
    pub kid: String,
    pub n: String,
    pub e: String,
}

impl ClerkService {
    pub fn new(secret_key: &str) -> Self {
        Self {
            client: Client::new(),
            secret_key: secret_key.to_string(),
        }
    }

    pub async fn get_user(&self, user_id: &str) -> AppResult<ClerkUser> {
        let url = format!("https://api.clerk.com/v1/users/{}", user_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.secret_key))
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Clerk API error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Clerk API error {}: {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse Clerk response: {}", e)))
    }

    pub async fn get_jwks(&self) -> AppResult<serde_json::Value> {
        let response = self
            .client
            .get("https://api.clerk.com/v1/jwks")
            .header("Authorization", format!("Bearer {}", self.secret_key))
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Clerk JWKS error: {}", e)))?;

        response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse JWKS: {}", e)))
    }

    pub fn get_primary_email(user: &ClerkUser) -> Option<String> {
        user.email_addresses.first().map(|e| e.email_address.clone())
    }

    pub fn get_full_name(user: &ClerkUser) -> Option<String> {
        match (&user.first_name, &user.last_name) {
            (Some(first), Some(last)) => Some(format!("{} {}", first, last)),
            (Some(first), None) => Some(first.clone()),
            (None, Some(last)) => Some(last.clone()),
            (None, None) => None,
        }
    }
}
