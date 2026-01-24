use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct EasyPostService {
    client: Client,
    api_key: String,
}

#[derive(Debug, Serialize)]
pub struct CreateTrackerRequest {
    pub tracker: TrackerParams,
}

#[derive(Debug, Serialize)]
pub struct TrackerParams {
    pub tracking_code: String,
    pub carrier: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Tracker {
    pub id: String,
    pub tracking_code: String,
    pub status: String,
    pub status_detail: Option<String>,
    pub carrier: String,
    pub est_delivery_date: Option<String>,
    pub tracking_details: Vec<TrackingDetail>,
}

#[derive(Debug, Deserialize)]
pub struct TrackingDetail {
    pub datetime: String,
    pub message: String,
    pub status: String,
    pub source: Option<String>,
    pub tracking_location: Option<TrackingLocation>,
}

#[derive(Debug, Deserialize)]
pub struct TrackingLocation {
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    pub zip: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EasyPostWebhookEvent {
    pub id: String,
    pub description: String,
    pub result: Tracker,
}

impl EasyPostService {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
        }
    }

    pub async fn create_tracker(
        &self,
        tracking_code: &str,
        carrier: Option<&str>,
    ) -> AppResult<Tracker> {
        let request = CreateTrackerRequest {
            tracker: TrackerParams {
                tracking_code: tracking_code.to_string(),
                carrier: carrier.map(|s| s.to_string()),
            },
        };

        let response = self
            .client
            .post("https://api.easypost.com/v2/trackers")
            .basic_auth(&self.api_key, Option::<&str>::None)
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("EasyPost API error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "EasyPost API error {}: {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse EasyPost response: {}", e)))
    }

    pub async fn get_tracker(&self, tracker_id: &str) -> AppResult<Tracker> {
        let url = format!("https://api.easypost.com/v2/trackers/{}", tracker_id);

        let response = self
            .client
            .get(&url)
            .basic_auth(&self.api_key, Option::<&str>::None)
            .send()
            .await
            .map_err(|e| AppError::ExternalService(format!("EasyPost API error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::ExternalService(format!(
                "EasyPost API error {}: {}",
                status, body
            )));
        }

        response
            .json()
            .await
            .map_err(|e| AppError::ExternalService(format!("Failed to parse EasyPost response: {}", e)))
    }

    pub fn map_status_to_order_status(easypost_status: &str) -> &'static str {
        match easypost_status {
            "delivered" => "delivered",
            "in_transit" | "out_for_delivery" => "shipped",
            "pre_transit" => "processing",
            "failure" | "error" => "cancelled",
            _ => "shipped",
        }
    }
}
