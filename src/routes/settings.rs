use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::models::{ArtistInfo, Setting};
use crate::routes::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/artist", get(get_artist_info))
        .route("/site", get(get_site_settings))
}

async fn get_artist_info(State(state): State<AppState>) -> AppResult<Json<ArtistInfo>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let info = Setting::get_artist_info(&conn).await?;
    Ok(Json(info))
}

#[derive(Serialize)]
pub struct SiteSettings {
    pub favicon: Option<String>,
}

async fn get_site_settings(State(state): State<AppState>) -> AppResult<Json<SiteSettings>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let favicon = Setting::get(&conn, "site_favicon").await?;
    Ok(Json(SiteSettings { favicon }))
}
