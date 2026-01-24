use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;

use crate::models::User;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClerkClaims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub azp: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: String,
    pub clerk_id: String,
    pub email: String,
    pub name: Option<String>,
    pub is_admin: bool,
}

impl From<User> for AuthUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            clerk_id: user.clerk_id,
            email: user.email,
            name: user.name,
            is_admin: user.is_admin,
        }
    }
}

pub async fn auth_middleware(
    State(pool): State<SqlitePool>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));

    let token = match auth_header {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Missing authorization header"})),
            )
                .into_response();
        }
    };

    // Decode the JWT without verification for now (in production, use JWKS)
    let mut validation = Validation::new(Algorithm::RS256);
    validation.insecure_disable_signature_validation();
    validation.validate_exp = true;

    let claims = match decode::<ClerkClaims>(
        token,
        &DecodingKey::from_secret(&[]),
        &validation,
    ) {
        Ok(data) => data.claims,
        Err(e) => {
            tracing::warn!("JWT decode error: {}", e);
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid token"})),
            )
                .into_response();
        }
    };

    let user = match User::find_by_clerk_id(&pool, &claims.sub).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "User not found"})),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Internal server error"})),
            )
                .into_response();
        }
    };

    req.extensions_mut().insert(AuthUser::from(user));
    next.run(req).await
}

pub async fn require_admin(
    State(_pool): State<SqlitePool>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let auth_user = req.extensions().get::<AuthUser>();

    match auth_user {
        Some(user) if user.is_admin => next.run(req).await,
        Some(_) => (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Admin access required"})),
        )
            .into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Authentication required"})),
        )
            .into_response(),
    }
}
