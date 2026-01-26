pub mod admin;
pub mod auth;
pub mod cart;
pub mod newsletter;
pub mod orders;
pub mod products;
pub mod settings;
pub mod webhooks;

use axum::{middleware, response::IntoResponse, Router};
use axum::http::StatusCode;
use libsql::Database;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::middleware::auth::auth_middleware;
use crate::services::{ClerkService, EmailService, ResendService, ShippoService, StripeService};
use crate::storage::StorageBackend;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub config: Config,
    pub clerk: ClerkService,
    pub stripe: StripeService,
    pub shippo: ShippoService,
    pub email: Option<EmailService>,
    pub resend: Option<ResendService>,
    pub storage: Arc<dyn StorageBackend>,
}

pub fn create_router(state: AppState) -> Router {
    let public_routes = Router::new()
        .merge(products::public_routes())
        .merge(auth::routes())
        .merge(webhooks::routes())
        .merge(settings::routes())
        .merge(newsletter::routes());

    let protected_routes = Router::new()
        .merge(orders::routes())
        .merge(cart::routes())
        .layer(middleware::from_fn_with_state(state.db.clone(), auth_middleware));

    let admin_routes = admin::routes(state.db.clone(), state.config.testing_mode);

    if state.config.testing_mode {
        tracing::warn!("⚠️  TESTING MODE ENABLED - Admin auth is disabled!");
    }

    Router::new()
        .nest("/api", public_routes)
        .nest("/api", protected_routes)
        .nest("/gallium", admin_routes)
        .nest_service("/uploads", ServeDir::new(&state.config.upload_dir))
        .fallback_service(
            ServeDir::new("static").fallback(ServeFile::new("static/index.html"))
        )
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
