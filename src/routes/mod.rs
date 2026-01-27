pub mod admin;
pub mod auth;
pub mod cart;
pub mod newsletter;
pub mod orders;
pub mod products;
pub mod settings;
pub mod shipping;
pub mod webhooks;

use axum::{middleware, Router};
use libsql::Database;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::middleware::auth::auth_middleware;
use crate::middleware::rate_limit::rate_limit_middleware;
use crate::services::{ClerkService, EmailService, JwksVerifier, RateLimiter, ResendService, ShippoService, StripeService};
use crate::storage::StorageBackend;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub config: Config,
    pub clerk: ClerkService,
    pub jwks: JwksVerifier,
    pub stripe: StripeService,
    pub shippo: ShippoService,
    pub email: Option<EmailService>,
    pub resend: Option<ResendService>,
    pub storage: Arc<dyn StorageBackend>,
    pub rate_limiter: Option<RateLimiter>,
}

pub fn create_router(state: AppState) -> Router {
    // Log rate limiting status
    if state.rate_limiter.is_some() {
        tracing::info!(
            "Distributed rate limiting enabled (Upstash): {} requests/minute",
            state.config.rate_limit_general
        );
    } else {
        tracing::warn!("Rate limiting disabled (no Upstash Redis configured)");
    }

    // Webhook routes (exempt from rate limiting)
    let webhook_routes = Router::new()
        .merge(webhooks::routes());

    // Public routes
    let public_routes = Router::new()
        .merge(products::public_routes())
        .merge(auth::routes())
        .merge(settings::routes())
        .merge(newsletter::routes())
        .merge(shipping::routes());

    let protected_routes = Router::new()
        .merge(orders::routes())
        .merge(cart::routes())
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    if state.config.testing_mode {
        tracing::warn!("TESTING MODE ENABLED - Admin auth is disabled!");
    }

    let admin_routes = admin::routes(state.clone());

    // Rate-limited API routes
    let rate_limited_api = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit_middleware));

    Router::new()
        .nest("/api/webhooks", webhook_routes) // Webhooks exempt from rate limiting
        .nest("/api", rate_limited_api)
        .nest("/gallium", admin_routes)
        .nest_service("/uploads", ServeDir::new(&state.config.upload_dir))
        .fallback_service(
            ServeDir::new("static").fallback(ServeFile::new("static/index.html"))
        )
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
