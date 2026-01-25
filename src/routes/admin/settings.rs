use axum::{
    extract::{Multipart, State},
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::models::{ArtistInfo, Setting};
use crate::routes::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/settings/artist", get(get_artist_info))
        .route("/settings/artist", put(update_artist_info))
        .route("/settings/artist/image", put(upload_artist_image))
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
