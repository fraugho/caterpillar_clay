use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::models::{Product, ProductImage, ProductNotification, ProductStyle};
use crate::routes::AppState;

#[derive(Serialize)]
pub struct StyleResponse {
    pub id: String,
    pub name: String,
    pub stock_quantity: i64,
    pub image_id: Option<String>,
    pub image_index: Option<usize>,
}

#[derive(Serialize)]
pub struct ProductResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub price_cents: i32,
    pub price: f64,
    pub images: Vec<String>,
    pub image_ids: Vec<String>,
    pub stock_quantity: i32,
    pub styles: Vec<StyleResponse>,
}

impl ProductResponse {
    fn from_product(
        product: Product,
        images: Vec<ProductImage>,
        styles: Vec<ProductStyle>,
        state: &AppState,
    ) -> Self {
        let image_ids: Vec<String> = images.iter().map(|img| img.id.clone()).collect();
        let image_urls: Vec<String> = images
            .iter()
            .map(|img| {
                if img.image_path.starts_with("http") {
                    img.image_path.clone()
                } else {
                    state.storage.public_url(&img.image_path)
                }
            })
            .collect();

        let style_responses: Vec<StyleResponse> = styles
            .into_iter()
            .map(|style| {
                let image_index = style
                    .image_id
                    .as_ref()
                    .and_then(|iid| image_ids.iter().position(|id| id == iid));
                StyleResponse {
                    id: style.id,
                    name: style.name,
                    stock_quantity: style.stock_quantity,
                    image_id: style.image_id,
                    image_index,
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
            image_ids,
            stock_quantity: product.stock_quantity,
            styles: style_responses,
        }
    }
}

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/products", get(list_products))
        .route("/products/{id}", get(get_product))
        .route("/products/{id}/notify", post(subscribe_notification))
}

async fn list_products(State(state): State<AppState>) -> AppResult<Json<Vec<ProductResponse>>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let products = Product::list_active(&conn).await?;

    let mut responses = Vec::new();
    for product in products {
        let images = ProductImage::list_by_product(&conn, &product.id).await?;
        let styles = ProductStyle::get_by_product(&conn, &product.id).await?;
        responses.push(ProductResponse::from_product(product, images, styles, &state));
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
    let styles = ProductStyle::get_by_product(&conn, &id).await?;

    Ok(Json(ProductResponse::from_product(product, images, styles, &state)))
}

#[derive(Deserialize)]
pub struct NotifyRequest {
    pub email: String,
    #[serde(default)]
    pub style_ids: Vec<String>,
}

#[derive(Serialize)]
pub struct NotifyResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subscribed_styles: Vec<String>,
}

async fn subscribe_notification(
    State(state): State<AppState>,
    Path(product_id): Path<String>,
    Json(payload): Json<NotifyRequest>,
) -> AppResult<Json<NotifyResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    // Verify product exists
    let _product = Product::find_by_id(&conn, &product_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    let mut subscribed_styles = Vec::new();

    if payload.style_ids.is_empty() {
        // Subscribe to the whole product (legacy behavior for products without styles)
        ProductNotification::subscribe(&conn, &payload.email, &product_id, None).await?;
    } else {
        // Subscribe to specific styles
        for style_id in &payload.style_ids {
            // Verify style exists and belongs to this product
            if let Some(style) = ProductStyle::get_by_id(&conn, style_id).await? {
                if style.product_id == product_id && style.stock_quantity == 0 {
                    ProductNotification::subscribe(&conn, &payload.email, &product_id, Some(style_id))
                        .await?;
                    subscribed_styles.push(style.name);
                }
            }
        }
    }

    Ok(Json(NotifyResponse {
        success: true,
        message: if subscribed_styles.is_empty() {
            "You'll be notified when this item is back in stock".to_string()
        } else {
            format!(
                "You'll be notified when {} {} back in stock",
                subscribed_styles.join(", "),
                if subscribed_styles.len() == 1 { "is" } else { "are" }
            )
        },
        subscribed_styles,
    }))
}
