use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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

#[derive(Debug, Serialize)]
struct CreateFileRequest {
    name: String,
    mime_type: String,
    size: i64,
    checksum_sha256_base64: String,
    upload: FileUploadConfig,
}

#[derive(Debug, Serialize)]
struct FileUploadConfig {
    service: String,
    is_uploaded: bool,
}

#[derive(Debug, Deserialize)]
struct FileResponse {
    id: String,
    upload: FileUploadInfo,
}

#[derive(Debug, Deserialize)]
struct FileUploadInfo {
    url: String,
    headers: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize)]
struct UpdateProductMedia {
    medias: Vec<String>,
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

    /// Upload an image to a Polar product
    pub async fn upload_product_image(
        &self,
        product_id: &str,
        filename: &str,
        mime_type: &str,
        data: &[u8],
    ) -> AppResult<()> {
        // Step 1: Calculate SHA256 checksum
        let mut hasher = Sha256::new();
        hasher.update(data);
        let checksum = hasher.finalize();
        let checksum_base64 = BASE64.encode(checksum);

        // Step 2: Request upload URL from Polar
        let create_request = CreateFileRequest {
            name: filename.to_string(),
            mime_type: mime_type.to_string(),
            size: data.len() as i64,
            checksum_sha256_base64: checksum_base64.clone(),
            upload: FileUploadConfig {
                service: "product_media".to_string(),
                is_uploaded: false,
            },
        };

        let response = self
            .client
            .post(format!("{}/files/", POLAR_API_URL))
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&create_request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Polar file API error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Polar file create error {}: {}",
                status, body
            )));
        }

        let file_response: FileResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse file response: {}", e)))?;

        // Step 3: Upload file to S3 with checksum header
        let mut upload_request = self
            .client
            .put(&file_response.upload.url)
            .header("Content-Type", mime_type)
            .header("x-amz-checksum-sha256", &checksum_base64);

        // Add any additional headers from Polar
        for (key, value) in &file_response.upload.headers {
            upload_request = upload_request.header(key, value);
        }

        let upload_response = upload_request
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("S3 upload error: {}", e)))?;

        if !upload_response.status().is_success() {
            let status = upload_response.status();
            let body = upload_response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "S3 upload error {}: {}",
                status, body
            )));
        }

        // Step 4: Mark file as uploaded
        let complete_response = self
            .client
            .post(format!("{}/files/{}/uploaded", POLAR_API_URL, file_response.id))
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Polar file complete error: {}", e)))?;

        if !complete_response.status().is_success() {
            let status = complete_response.status();
            let body = complete_response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Polar file complete error {}: {}",
                status, body
            )));
        }

        // Step 5: Attach media to product
        let media_request = UpdateProductMedia {
            medias: vec![file_response.id],
        };

        let product_response = self
            .client
            .patch(format!("{}/products/{}", POLAR_API_URL, product_id))
            .header("Authorization", format!("Bearer {}", self.access_token))
            .json(&media_request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Polar product update error: {}", e)))?;

        if !product_response.status().is_success() {
            let status = product_response.status();
            let body = product_response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Polar product media update error {}: {}",
                status, body
            )));
        }

        Ok(())
    }
}
