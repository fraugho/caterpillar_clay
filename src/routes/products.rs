use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::models::{Product, ProductImage};
use crate::routes::AppState;

#[derive(Serialize)]
pub struct ProductResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub price_cents: i32,
    pub price: f64,
    pub images: Vec<String>,
    pub stock_quantity: i32,
}

impl ProductResponse {
    fn from_product(product: Product, images: Vec<ProductImage>, state: &AppState) -> Self {
        let image_urls: Vec<String> = images
            .into_iter()
            .map(|img| {
                if img.image_path.starts_with("http") {
                    img.image_path
                } else {
                    state.storage.public_url(&img.image_path)
                }
            })
            .collect();

        Self {
            id: product.id,
            name: product.name,
            description: product.description,
            price_cents: product.price_cents,
            price: product.price_cents as f64 / 100.0,
            images: image_urls,
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

    let mut responses = Vec::new();
    for product in products {
        let images = ProductImage::list_by_product(&conn, &product.id).await?;
        responses.push(ProductResponse::from_product(product, images, &state));
    }

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

    let images = ProductImage::list_by_product(&conn, &id).await?;

    Ok(Json(ProductResponse::from_product(product, images, &state)))
}
