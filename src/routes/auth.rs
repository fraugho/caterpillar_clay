use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::models::{CreateUser, User};
use crate::routes::AppState;
use crate::services::clerk::ClerkService;

#[derive(Deserialize)]
pub struct AuthCallback {
    pub user_id: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/callback", get(auth_callback))
        .route("/auth/sync", post(sync_user))
        .route("/auth/logout", post(logout))
}

async fn auth_callback(
    State(state): State<AppState>,
    Query(params): Query<AuthCallback>,
) -> impl IntoResponse {
    if let Some(user_id) = params.user_id {
        // Sync user from Clerk
        if let Ok(clerk_user) = state.clerk.get_user(&user_id).await {
            let email = ClerkService::get_primary_email(&clerk_user)
                .unwrap_or_else(|| "unknown@example.com".to_string());
            let name = ClerkService::get_full_name(&clerk_user);

            if let Ok(conn) = state.db.connect() {
                let _ = User::upsert(
                    &conn,
                    CreateUser {
                        clerk_id: clerk_user.id,
                        email,
                        name,
                    },
                )
                .await;
            }
        }
    }

    Redirect::to("/")
}

#[derive(Deserialize)]
pub struct SyncUserRequest {
    pub clerk_id: String,
}

async fn sync_user(
    State(state): State<AppState>,
    Json(payload): Json<SyncUserRequest>,
) -> AppResult<Json<User>> {
    let clerk_user = state.clerk.get_user(&payload.clerk_id).await?;

    let email = ClerkService::get_primary_email(&clerk_user)
        .unwrap_or_else(|| "unknown@example.com".to_string());
    let name = ClerkService::get_full_name(&clerk_user);

    let conn = state.db.connect().map_err(AppError::from)?;
    let user = User::upsert(
        &conn,
        CreateUser {
            clerk_id: clerk_user.id,
            email,
            name,
        },
    )
    .await?;

    Ok(Json(user))
}

async fn logout() -> impl IntoResponse {
    // Client-side handles Clerk signout
    // This endpoint can be used to clear any server-side session data
    Json(serde_json::json!({"success": true}))
}
