pub mod dashboard;
pub mod orders;
pub mod products;

use axum::{middleware, Router};
use sqlx::SqlitePool;
use tower_http::services::ServeDir;

use crate::middleware::auth::{auth_middleware, require_admin};
use crate::routes::AppState;

pub fn routes(pool: SqlitePool) -> Router<AppState> {
    let api_routes = Router::new()
        .merge(products::routes())
        .merge(orders::routes())
        .merge(dashboard::routes())
        .layer(middleware::from_fn_with_state(pool.clone(), require_admin))
        .layer(middleware::from_fn_with_state(pool, auth_middleware));

    Router::new()
        .nest("/api", api_routes)
        .nest_service("/", ServeDir::new("static/admin"))
}
