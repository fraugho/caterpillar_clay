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
    let api_routes = if state.config.testing_mode {
        // Skip auth in testing mode
        Router::new()
            .merge(products::routes())
            .merge(orders::routes())
            .merge(dashboard::routes())
            .merge(settings::routes())
            .merge(newsletter::routes())
    } else {
        Router::new()
            .merge(products::routes())
            .merge(orders::routes())
            .merge(dashboard::routes())
            .merge(settings::routes())
            .merge(newsletter::routes())
            .layer(middleware::from_fn(require_admin))
            .layer(middleware::from_fn_with_state(state, auth_middleware))
    };

    Router::new()
        .nest("/api", api_routes)
        .fallback_service(ServeDir::new("static/gallium"))
}
