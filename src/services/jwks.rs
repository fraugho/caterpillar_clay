use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwksResponse {
    pub keys: Vec<Jwk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwk {
    pub kty: String,
    #[serde(rename = "use")]
    pub use_: Option<String>,
    pub kid: String,
    pub n: String,
    pub e: String,
    pub alg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClerkClaims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub azp: Option<String>,
}

pub struct JwksVerifier {
    client: Client,
    jwks_url: String,
    keys: Arc<RwLock<HashMap<String, DecodingKey>>>,
    max_retries: u32,
}

impl JwksVerifier {
    pub fn new(jwks_url: &str) -> Self {
        Self {
            client: Client::new(),
            jwks_url: jwks_url.to_string(),
            keys: Arc::new(RwLock::new(HashMap::new())),
            max_retries: 3,
        }
    }

    /// Fetch JWKS from Clerk and cache the keys
    pub async fn refresh_keys(&self) -> AppResult<()> {
        tracing::info!("Fetching JWKS from {}", self.jwks_url);

        let response = self
            .client
            .get(&self.jwks_url)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to fetch JWKS: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "JWKS fetch failed {}: {}",
                status, body
            )));
        }

        let jwks: JwksResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse JWKS: {}", e)))?;

        let mut keys = self.keys.write().await;
        keys.clear();

        for jwk in jwks.keys {
            if jwk.kty == "RSA" {
                match DecodingKey::from_rsa_components(&jwk.n, &jwk.e) {
                    Ok(key) => {
                        keys.insert(jwk.kid.clone(), key);
                        tracing::debug!("Cached JWKS key: {}", jwk.kid);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse JWK {}: {}", jwk.kid, e);
                    }
                }
            }
        }

        tracing::info!("Cached {} JWKS keys", keys.len());
        Ok(())
    }

    /// Verify a JWT token and return the claims
    pub async fn verify_token(&self, token: &str) -> AppResult<ClerkClaims> {
        // Try verification with cached keys first
        match self.try_verify(token).await {
            Ok(claims) => return Ok(claims),
            Err(_) => {
                // Key might be rotated, try refreshing
                tracing::debug!("Token verification failed, attempting JWKS refresh");
            }
        }

        // Retry with refresh up to max_retries times
        for attempt in 1..=self.max_retries {
            if let Err(e) = self.refresh_keys().await {
                tracing::warn!("JWKS refresh attempt {} failed: {}", attempt, e);
                if attempt == self.max_retries {
                    return Err(AppError::ExternalService(
                        "Failed to refresh JWKS after max retries".to_string(),
                    ));
                }
                continue;
            }

            match self.try_verify(token).await {
                Ok(claims) => return Ok(claims),
                Err(e) => {
                    if attempt == self.max_retries {
                        return Err(e);
                    }
                    tracing::warn!("Verification attempt {} failed: {}", attempt, e);
                }
            }
        }

        Err(AppError::ExternalService(
            "Token verification failed after retries".to_string(),
        ))
    }

    async fn try_verify(&self, token: &str) -> AppResult<ClerkClaims> {
        // Decode header to get the key ID
        let header = decode_header(token)
            .map_err(|e| AppError::ExternalService(format!("Invalid token header: {}", e)))?;

        let kid = header
            .kid
            .ok_or_else(|| AppError::ExternalService("Token missing kid".to_string()))?;

        // Get the key from cache
        let keys = self.keys.read().await;
        let key = keys
            .get(&kid)
            .ok_or_else(|| AppError::ExternalService(format!("Unknown key ID: {}", kid)))?;

        // Verify and decode the token
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;

        let token_data = decode::<ClerkClaims>(token, key, &validation)
            .map_err(|e| AppError::ExternalService(format!("Token verification failed: {}", e)))?;

        Ok(token_data.claims)
    }

    /// Initialize by fetching keys (call on startup)
    pub async fn initialize(&self) -> AppResult<()> {
        self.refresh_keys().await
    }
}

impl Clone for JwksVerifier {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            jwks_url: self.jwks_url.clone(),
            keys: self.keys.clone(),
            max_retries: self.max_retries,
        }
    }
}
