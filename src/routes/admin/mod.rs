pub mod dashboard;
pub mod newsletter;
pub mod orders;
pub mod products;
pub mod settings;

use axum::{middleware, Router};
use tower_http::services::ServeDir;

use crate::middleware::auth::{auth_middleware, require_admin};
use crate::routes::AppState;

pub fn routes(state: AppState) -> Router<AppState> {
    // Skip auth only in local testing mode (not cloud)
    let skip_auth = state.config.testing_mode && !state.config.deploy_mode.is_cloud();

    let api_routes = Router::new()
        .merge(products::routes())
        .merge(orders::routes())
        .merge(dashboard::routes())
        .merge(settings::routes())
        .merge(newsletter::routes());

    let base_router = Router::new()
        .nest("/api", api_routes)
        .fallback_service(ServeDir::new("static/gallium"));

    if skip_auth {
        base_router
    } else {
        // Apply auth to entire admin router (API + static files)
        base_router
            .layer(middleware::from_fn(require_admin))
            .layer(middleware::from_fn_with_state(state, auth_middleware))
    }
}
