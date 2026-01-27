pub mod dashboard;
pub mod newsletter;
pub mod orders;
pub mod products;
pub mod settings;

use axum::{
    extract::Path,
    http::{header, StatusCode},
    middleware,
    response::IntoResponse,
    routing::get,
    Router,
};

use crate::middleware::auth::{auth_middleware, require_admin};
use crate::routes::AppState;

async fn serve_admin_static(path: Option<Path<String>>) -> impl IntoResponse {
    let file_path = match &path {
        Some(Path(p)) if !p.is_empty() => format!("static/gallium/{}", p),
        _ => "static/gallium/index.html".to_string(),
    };

    match tokio::fs::read(&file_path).await {
        Ok(contents) => {
            let mime = if file_path.ends_with(".html") {
                "text/html"
            } else if file_path.ends_with(".js") {
                "application/javascript"
            } else if file_path.ends_with(".css") {
                "text/css"
            } else if file_path.ends_with(".png") {
                "image/png"
            } else if file_path.ends_with(".svg") {
                "image/svg+xml"
            } else if file_path.ends_with(".ico") {
                "image/x-icon"
            } else {
                "application/octet-stream"
            };
            (StatusCode::OK, [(header::CONTENT_TYPE, mime)], contents).into_response()
        }
        Err(_) => {
            // SPA fallback - serve index.html for client-side routing
            match tokio::fs::read("static/gallium/index.html").await {
                Ok(contents) => {
                    (StatusCode::OK, [(header::CONTENT_TYPE, "text/html")], contents).into_response()
                }
                Err(_) => StatusCode::NOT_FOUND.into_response(),
            }
        }
    }
}

pub fn routes(state: AppState) -> Router<AppState> {
    // Skip auth only in local testing mode (not cloud)
    let skip_auth = state.config.testing_mode && !state.config.deploy_mode.is_cloud();

    let api_routes = Router::new()
        .merge(products::routes())
        .merge(orders::routes())
        .merge(dashboard::routes())
        .merge(settings::routes())
        .merge(newsletter::routes());

    // Serve static files through route handlers (not fallback_service)
    // so middleware applies properly
    let static_routes = Router::new()
        .route("/", get(serve_admin_static))
        .route("/{*path}", get(serve_admin_static));

    let base_router = Router::new()
        .nest("/api", api_routes)
        .merge(static_routes);

    if skip_auth {
        base_router
    } else {
        // Apply auth to entire admin router (API + static files)
        base_router
            .layer(middleware::from_fn(require_admin))
            .layer(middleware::from_fn_with_state(state, auth_middleware))
    }
}
