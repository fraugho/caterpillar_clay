use axum::{
    extract::{Multipart, State},
    routing::{get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::models::{ArtistInfo, Setting, ShopAddress};
use crate::routes::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/settings/artist", get(get_artist_info))
        .route("/settings/artist", put(update_artist_info))
        .route("/settings/artist/image", put(upload_artist_image))
        .route("/settings/favicon", get(get_favicon))
        .route("/settings/favicon", put(upload_favicon))
        .route("/settings/shipping", get(get_shipping_settings))
        .route("/settings/shipping/address", put(update_shop_address))
        .route("/settings/shipping/units", put(update_unit_system))
}

async fn get_artist_info(State(state): State<AppState>) -> AppResult<Json<ArtistInfo>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let info = Setting::get_artist_info(&conn).await?;
    Ok(Json(info))
}

#[derive(Deserialize)]
pub struct UpdateArtistRequest {
    pub description: String,
}

async fn update_artist_info(
    State(state): State<AppState>,
    Json(payload): Json<UpdateArtistRequest>,
) -> AppResult<Json<ArtistInfo>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    Setting::set(&conn, "artist_description", &payload.description).await?;

    let info = Setting::get_artist_info(&conn).await?;
    Ok(Json(info))
}

async fn upload_artist_image(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<Json<ArtistInfo>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::BadRequest(format!("Failed to process upload: {}", e))
    })? {
        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "artist.jpg".to_string());

        let data = field.bytes().await.map_err(|e| {
            AppError::BadRequest(format!("Failed to read upload: {}", e))
        })?;

        // Upload to storage in artist folder
        let path = state
            .storage
            .upload_to_folder("artist", &filename, &data)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        // Get public URL and save to settings
        let image_url = state.storage.public_url(&path);
        Setting::set(&conn, "artist_image", &image_url).await?;
    }

    let info = Setting::get_artist_info(&conn).await?;
    Ok(Json(info))
}

#[derive(serde::Serialize)]
pub struct FaviconResponse {
    pub url: Option<String>,
}

async fn get_favicon(State(state): State<AppState>) -> AppResult<Json<FaviconResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let url = Setting::get(&conn, "site_favicon").await?;
    Ok(Json(FaviconResponse { url }))
}

async fn upload_favicon(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> AppResult<Json<FaviconResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::BadRequest(format!("Failed to process upload: {}", e))
    })? {
        let data = field.bytes().await.map_err(|e| {
            AppError::BadRequest(format!("Failed to read upload: {}", e))
        })?;

        // Upload to storage in site folder
        let path = state
            .storage
            .upload_to_folder("site", "favicon.png", &data)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        // Get public URL and save to settings
        let favicon_url = state.storage.public_url(&path);
        Setting::set(&conn, "site_favicon", &favicon_url).await?;

        return Ok(Json(FaviconResponse { url: Some(favicon_url) }));
    }

    Err(AppError::BadRequest("No file uploaded".to_string()))
}

// ============ SHIPPING SETTINGS ============

#[derive(Serialize)]
pub struct ShippingSettingsResponse {
    pub address: Option<ShopAddress>,
    pub unit_system: String,
}

async fn get_shipping_settings(State(state): State<AppState>) -> AppResult<Json<ShippingSettingsResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let address = Setting::get_shop_address(&conn).await?;
    let unit_system = Setting::get_unit_system(&conn).await?;
    Ok(Json(ShippingSettingsResponse { address, unit_system }))
}

async fn update_shop_address(
    State(state): State<AppState>,
    Json(payload): Json<ShopAddress>,
) -> AppResult<Json<serde_json::Value>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    Setting::set(&conn, "shop_name", &payload.name).await?;
    Setting::set(&conn, "shop_street1", &payload.street1).await?;
    Setting::set(&conn, "shop_street2", &payload.street2.unwrap_or_default()).await?;
    Setting::set(&conn, "shop_city", &payload.city).await?;
    Setting::set(&conn, "shop_state", &payload.state).await?;
    Setting::set(&conn, "shop_zip", &payload.zip).await?;
    Setting::set(&conn, "shop_country", &payload.country).await?;
    Setting::set(&conn, "shop_phone", &payload.phone.unwrap_or_default()).await?;

    Ok(Json(serde_json::json!({"success": true})))
}

#[derive(Deserialize)]
pub struct UnitSystemRequest {
    pub unit_system: String,
}

async fn update_unit_system(
    State(state): State<AppState>,
    Json(payload): Json<UnitSystemRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    Setting::set(&conn, "shipping_unit_system", &payload.unit_system).await?;
    Ok(Json(serde_json::json!({"success": true})))
}
