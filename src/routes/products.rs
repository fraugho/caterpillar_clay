use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::models::Product;
use crate::routes::AppState;

#[derive(Serialize)]
pub struct ProductResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub price_cents: i32,
    pub price: f64,
    pub image_url: Option<String>,
    pub stock_quantity: i32,
}

impl ProductResponse {
    fn from_product(product: Product, state: &AppState) -> Self {
        let image_url = product.image_path.map(|p| {
            if p.starts_with("http") {
                p
            } else {
                state.storage.public_url(&p)
            }
        });

        Self {
            id: product.id,
            name: product.name,
            description: product.description,
            price_cents: product.price_cents,
            price: product.price_cents as f64 / 100.0,
            image_url,
            stock_quantity: product.stock_quantity,
        }
    }
}

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/products", get(list_products))
        .route("/products/{id}", get(get_product))
}

async fn list_products(State(state): State<AppState>) -> AppResult<Json<Vec<ProductResponse>>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let products = Product::list_active(&conn).await?;

    let responses: Vec<ProductResponse> = products
        .into_iter()
        .map(|p| ProductResponse::from_product(p, &state))
        .collect();

    Ok(Json(responses))
}

async fn get_product(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<ProductResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let product = Product::find_by_id(&conn, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    if !product.is_active {
        return Err(AppError::NotFound("Product not found".to_string()));
    }

    Ok(Json(ProductResponse::from_product(product, &state)))
}
