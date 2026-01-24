use axum::{
    extract::{Multipart, Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::models::{CreateProduct, Product, UpdateProduct};
use crate::routes::AppState;

#[derive(Serialize)]
pub struct AdminProductResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub price_cents: i32,
    pub price: f64,
    pub image_path: Option<String>,
    pub image_url: Option<String>,
    pub stock_quantity: i32,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl AdminProductResponse {
    fn from_product(product: Product, state: &AppState) -> Self {
        let image_url = product.image_path.clone().map(|p| {
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
            image_path: product.image_path,
            image_url,
            stock_quantity: product.stock_quantity,
            is_active: product.is_active,
            created_at: product.created_at,
            updated_at: product.updated_at,
        }
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/products", get(list_products))
        .route("/products", post(create_product))
        .route("/products/{id}", get(get_product))
        .route("/products/{id}", put(update_product))
        .route("/products/{id}", delete(delete_product))
        .route("/products/{id}/image", post(upload_image))
}

async fn list_products(State(state): State<AppState>) -> AppResult<Json<Vec<AdminProductResponse>>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let products = Product::list_all(&conn).await?;

    let responses: Vec<AdminProductResponse> = products
        .into_iter()
        .map(|p| AdminProductResponse::from_product(p, &state))
        .collect();

    Ok(Json(responses))
}

async fn get_product(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<AdminProductResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let product = Product::find_by_id(&conn, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    Ok(Json(AdminProductResponse::from_product(product, &state)))
}

async fn create_product(
    State(state): State<AppState>,
    Json(payload): Json<CreateProduct>,
) -> AppResult<Json<AdminProductResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let product = Product::create(&conn, payload).await?;

    Ok(Json(AdminProductResponse::from_product(product, &state)))
}

async fn update_product(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateProduct>,
) -> AppResult<Json<AdminProductResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    // Verify product exists
    Product::find_by_id(&conn, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    let product = Product::update(&conn, &id, payload).await?;

    Ok(Json(AdminProductResponse::from_product(product, &state)))
}

async fn delete_product(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    // Verify product exists
    let product = Product::find_by_id(&conn, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    // Delete image if exists
    if let Some(image_path) = &product.image_path {
        let _ = state.storage.delete(image_path).await;
    }

    Product::delete(&conn, &id).await?;

    Ok(Json(serde_json::json!({"deleted": true})))
}

async fn upload_image(
    State(state): State<AppState>,
    Path(id): Path<String>,
    mut multipart: Multipart,
) -> AppResult<Json<AdminProductResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    // Verify product exists
    let product = Product::find_by_id(&conn, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    // Delete old image if exists
    if let Some(old_path) = &product.image_path {
        let _ = state.storage.delete(old_path).await;
    }

    // Process upload
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::BadRequest(format!("Failed to process upload: {}", e))
    })? {
        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "image.jpg".to_string());

        let content_type = field.content_type().map(|s| s.to_string());

        // Validate content type
        if let Some(ref ct) = content_type {
            if !ct.starts_with("image/") {
                return Err(AppError::BadRequest("Only image files are allowed".to_string()));
            }
        }

        let data = field.bytes().await.map_err(|e| {
            AppError::BadRequest(format!("Failed to read upload: {}", e))
        })?;

        // Upload to storage
        let path = state
            .storage
            .upload(&filename, &data)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        // Update product with new image path
        let updated = Product::set_image(&conn, &id, &path).await?;

        return Ok(Json(AdminProductResponse::from_product(updated, &state)));
    }

    Err(AppError::BadRequest("No file uploaded".to_string()))
}
