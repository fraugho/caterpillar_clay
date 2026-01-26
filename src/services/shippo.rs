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
    pub data: ShippoTracking,
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
}
