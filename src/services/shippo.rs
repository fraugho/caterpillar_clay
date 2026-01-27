use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct ShippoService {
    client: Client,
    api_key: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterTrackingRequest {
    pub carrier: String,
    pub tracking_number: String,
}

#[derive(Debug, Deserialize)]
pub struct ShippoTracking {
    pub tracking_number: String,
    pub carrier: String,
    pub tracking_status: Option<TrackingStatus>,
    pub tracking_history: Vec<TrackingEvent>,
    pub eta: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TrackingStatus {
    pub status: String,
    pub status_details: Option<String>,
    pub status_date: Option<String>,
    pub location: Option<TrackingLocation>,
}

#[derive(Debug, Deserialize)]
pub struct TrackingEvent {
    pub status: String,
    pub status_details: Option<String>,
    pub status_date: Option<String>,
    pub location: Option<TrackingLocation>,
}

#[derive(Debug, Deserialize)]
pub struct TrackingLocation {
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ShippoWebhookEvent {
    pub event: String,
    #[serde(default)]
    pub test: bool,
    pub data: serde_json::Value,
}

impl ShippoWebhookEvent {
    pub fn as_tracking(&self) -> Option<ShippoTracking> {
        serde_json::from_value(self.data.clone()).ok()
    }

    pub fn as_transaction(&self) -> Option<ShippoTransactionWebhookData> {
        serde_json::from_value(self.data.clone()).ok()
    }
}

// ============ SHIPPING RATES TYPES ============

#[derive(Debug, Serialize)]
pub struct ShippoAddress {
    pub name: String,
    pub street1: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street2: Option<String>,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub country: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ShippoParcel {
    pub length: f64,
    pub width: f64,
    pub height: f64,
    pub distance_unit: String,
    pub weight: f64,
    pub mass_unit: String,
}

#[derive(Debug, Serialize)]
struct CreateShipmentRequest {
    address_from: ShippoAddress,
    address_to: ShippoAddress,
    parcels: Vec<ShippoParcel>,
    #[serde(rename = "async")]
    async_mode: bool,
}

#[derive(Debug, Deserialize)]
struct ShippoShipment {
    pub rates: Vec<ShippoRate>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShippoRate {
    pub object_id: String,
    pub provider: String,
    pub servicelevel: ShippoServiceLevel,
    pub amount: String,
    pub currency: String,
    pub estimated_days: Option<i32>,
    pub duration_terms: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShippoServiceLevel {
    pub name: String,
    pub token: String,
}

// ============ TRANSACTION/LABEL TYPES ============

#[derive(Debug, Serialize)]
struct CreateTransactionRequest {
    rate: String,
    label_file_type: String,
    #[serde(rename = "async")]
    async_mode: bool,
}

#[derive(Debug, Deserialize)]
pub struct ShippoTransaction {
    pub object_id: String,
    pub status: String,
    pub tracking_number: Option<String>,
    pub label_url: Option<String>,
    pub tracking_url_provider: Option<String>,
    pub rate: Option<String>,
    pub messages: Option<Vec<ShippoMessage>>,
}

#[derive(Debug, Deserialize)]
pub struct ShippoMessage {
    pub source: Option<String>,
    pub code: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ShippoTransactionWebhookData {
    pub object_id: String,
    pub status: String,
    pub tracking_number: Option<String>,
    pub label_url: Option<String>,
    pub rate: Option<String>,
}

impl ShippoService {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
        }
    }

    /// Register a tracking number to receive webhook updates
    pub async fn register_tracking(
        &self,
        tracking_number: &str,
        carrier: &str,
    ) -> AppResult<ShippoTracking> {
        let request = RegisterTrackingRequest {
            carrier: carrier.to_string(),
            tracking_number: tracking_number.to_string(),
        };

        let response = self
            .client
            .post("https://api.goshippo.com/tracks/")
            .header("Authorization", format!("ShippoToken {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Shippo API error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Shippo API error {}: {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse Shippo response: {}", e)))
    }

    /// Get tracking status for a shipment
    pub async fn get_tracking(
        &self,
        carrier: &str,
        tracking_number: &str,
    ) -> AppResult<ShippoTracking> {
        let url = format!(
            "https://api.goshippo.com/tracks/{}/{}",
            carrier, tracking_number
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("ShippoToken {}", self.api_key))
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Shippo API error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Shippo API error {}: {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse Shippo response: {}", e)))
    }

    /// Map Shippo tracking status to order status
    pub fn map_status_to_order_status(shippo_status: &str) -> &'static str {
        match shippo_status.to_uppercase().as_str() {
            "DELIVERED" => "delivered",
            "TRANSIT" => "shipped",
            "PRE_TRANSIT" => "processing",
            "RETURNED" | "FAILURE" => "cancelled",
            _ => "shipped",
        }
    }

    /// Get shipping rates for a shipment
    pub async fn get_rates(
        &self,
        from_address: ShippoAddress,
        to_address: ShippoAddress,
        parcels: Vec<ShippoParcel>,
    ) -> AppResult<Vec<ShippoRate>> {
        let request = CreateShipmentRequest {
            address_from: from_address,
            address_to: to_address,
            parcels,
            async_mode: false,
        };

        let response = self
            .client
            .post("https://api.goshippo.com/shipments/")
            .header("Authorization", format!("ShippoToken {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Shippo API error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Shippo API error {}: {}",
                status, body
            )));
        }

        let shipment: ShippoShipment = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse Shippo response: {}", e)))?;

        // Filter for USD rates and sort by price
        let mut rates: Vec<ShippoRate> = shipment
            .rates
            .into_iter()
            .filter(|r| r.currency == "USD")
            .collect();

        rates.sort_by(|a, b| {
            let a_amount: f64 = a.amount.parse().unwrap_or(f64::MAX);
            let b_amount: f64 = b.amount.parse().unwrap_or(f64::MAX);
            a_amount.partial_cmp(&b_amount).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(rates)
    }

    /// Purchase a shipping label using a rate object_id
    pub async fn purchase_label(&self, rate_id: &str) -> AppResult<ShippoTransaction> {
        let request = CreateTransactionRequest {
            rate: rate_id.to_string(),
            label_file_type: "PDF".to_string(),
            async_mode: false,
        };

        let response = self
            .client
            .post("https://api.goshippo.com/transactions/")
            .header("Authorization", format!("ShippoToken {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("Shippo API error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "Shippo API error {}: {}",
                status, body
            )));
        }

        let transaction: ShippoTransaction = response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse Shippo response: {}", e)))?;

        // Check for errors
        if transaction.status == "ERROR" {
            let error_msg = transaction
                .messages
                .as_ref()
                .and_then(|msgs| msgs.first())
                .and_then(|m| m.text.clone())
                .unwrap_or_else(|| "Label purchase failed".to_string());
            return Err(AppError::ExternalService(error_msg));
        }

        Ok(transaction)
    }
}
