use axum::{extract::State, routing::get, Json, Router};

use crate::error::{AppError, AppResult};
use crate::models::{ArtistInfo, Setting};
use crate::routes::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/artist", get(get_artist_info))
}

async fn get_artist_info(State(state): State<AppState>) -> AppResult<Json<ArtistInfo>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let info = Setting::get_artist_info(&conn).await?;
    Ok(Json(info))
}
