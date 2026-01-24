use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

const POLAR_API_URL: &str = "https://api.polar.sh/v1";

#[derive(Clone)]
pub struct PolarService {
    client: Client,
    access_token: String,
}

#[derive(Debug, Serialize)]
pub struct CreatePolarProduct {
    pub name: String,
    pub description: Option<String>,
    pub prices: Vec<PolarPrice>,
}

#[derive(Debug, Serialize)]
pub struct PolarPrice {
    #[serde(rename = "type")]
    pub price_type: String,
    pub amount_type: String,
    pub price_amount: i32,
    pub price_currency: String,
}

#[derive(Debug, Serialize)]
pub struct UpdatePolarProduct {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_archived: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct PolarProduct {
    pub id: String,
    pub name: String,
    pub prices: Vec<PolarPriceResponse>,
}

#[derive(Debug, Deserialize)]
pub struct PolarPriceResponse {
    pub id: String,
    pub price_amount: i32,
}

#[derive(Debug, Serialize)]
pub struct CreateCheckoutRequest {
    pub product_price_id: String,
    pub success_url: String,
    pub customer_email: Option<String>,
    pub metadata: CheckoutMetadata,
}

#[derive(Debug, Serialize)]
pub struct CheckoutMetadata {
    pub order_id: String,
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutResponse {
    pub id: String,
    pub url: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct PolarWebhookEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutCompletedData {
    pub id: String,
    pub status: String,
    pub metadata: Option<serde_json::Value>,
}

impl PolarService {
    pub fn new(access_token: &str) -> Self {
        Self {
            client: Client::new(),
            access_token: access_token.to_string(),
        }
    }

    pub async fn create_checkout(
        &self,
        product_price_id: &str,
        success_url: &str,
        customer_email: Option<&str>,
        order_id: Uuid,
        user_id: Option<Uuid>,
    ) -> AppResult<CheckoutResponse> {
        let request = CreateCheckoutRequest {
            product_price_id: product_price_id.to_string(),
            success_url: success_url.to_string(),
            customer_email: customer_email.map(|s| s.to_string()),
            metadata: CheckoutMetadata {
                order_id: order_id.to_string(),
                user_id: user_id.map(|u| u.to_string()),
            },
        };

        let response = self
            .client
            .post("https://api.polar.sh/v1/checkouts/custom")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Polar API error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Polar API error {}: {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse Polar response: {}", e)))
    }

    pub async fn get_checkout(&self, checkout_id: &str) -> AppResult<CheckoutResponse> {
        let url = format!("https://api.polar.sh/v1/checkouts/custom/{}", checkout_id);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Polar API error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Polar API error {}: {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse Polar response: {}", e)))
    }

    pub fn verify_webhook_signature(
        &self,
        _payload: &[u8],
        _signature: &str,
        _secret: &str,
    ) -> bool {
        // Polar uses HMAC-SHA256 for webhook signatures
        // In production, implement proper signature verification
        true
    }

    /// Create a product in Polar, returns (product_id, price_id)
    pub async fn create_product(
        &self,
        name: &str,
        description: Option<&str>,
        price_cents: i32,
    ) -> AppResult<(String, String)> {
        let request = CreatePolarProduct {
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            prices: vec![PolarPrice {
                price_type: "one_time".to_string(),
                amount_type: "fixed".to_string(),
                price_amount: price_cents,
                price_currency: "usd".to_string(),
            }],
        };

        let response = self
            .client
            .post(format!("{}/products/", POLAR_API_URL))
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Polar API error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Polar API error {}: {}",
                status, body
            )));
        }

        let product: PolarProduct = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse Polar response: {}", e)))?;

        let price_id = product
            .prices
            .first()
            .map(|p| p.id.clone())
            .ok_or_else(|| AppError::ExternalService("No price returned from Polar".to_string()))?;

        Ok((product.id, price_id))
    }

    /// Update a product in Polar
    pub async fn update_product(
        &self,
        product_id: &str,
        name: Option<&str>,
        description: Option<&str>,
    ) -> AppResult<()> {
        let request = UpdatePolarProduct {
            name: name.map(|s| s.to_string()),
            description: description.map(|s| s.to_string()),
            is_archived: None,
        };

        let response = self
            .client
            .patch(format!("{}/products/{}", POLAR_API_URL, product_id))
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Polar API error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Polar API error {}: {}",
                status, body
            )));
        }

        Ok(())
    }

    /// Archive a product in Polar (soft delete)
    pub async fn archive_product(&self, product_id: &str) -> AppResult<()> {
        let request = UpdatePolarProduct {
            name: None,
            description: None,
            is_archived: Some(true),
        };

        let response = self
            .client
            .patch(format!("{}/products/{}", POLAR_API_URL, product_id))
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Polar API error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Polar API error {}: {}",
                status, body
            )));
        }

        Ok(())
    }
}
