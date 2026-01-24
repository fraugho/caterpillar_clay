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
    pub polar_product_id: Option<String>,
    pub polar_price_id: Option<String>,
    pub created_ts: i64,
    pub updated_ts: i64,
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
            polar_product_id: product.polar_product_id,
            polar_price_id: product.polar_price_id,
            created_ts: product.created_ts,
            updated_ts: product.updated_ts,
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

    // Extract values for Polar sync before moving payload
    let name = payload.name.clone();
    let description = payload.description.clone();
    let price_cents = payload.price_cents;

    let mut product = Product::create(&conn, payload).await?;

    // Sync to Polar
    match state
        .polar
        .create_product(&name, description.as_deref(), price_cents)
        .await
    {
        Ok((polar_product_id, polar_price_id)) => {
            product = Product::set_polar_ids(&conn, &product.id, &polar_product_id, &polar_price_id).await?;
        }
        Err(e) => {
            tracing::warn!("Failed to sync product to Polar: {}", e);
        }
    }

    Ok(Json(AdminProductResponse::from_product(product, &state)))
}

async fn update_product(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateProduct>,
) -> AppResult<Json<AdminProductResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    // Verify product exists and get current state
    let current = Product::find_by_id(&conn, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    // Extract values for Polar sync
    let name_update = payload.name.clone();
    let desc_update = payload.description.clone();

    let product = Product::update(&conn, &id, payload).await?;

    // Sync to Polar if product is linked
    if let Some(polar_product_id) = &current.polar_product_id {
        if let Err(e) = state
            .polar
            .update_product(polar_product_id, name_update.as_deref(), desc_update.as_deref())
            .await
        {
            tracing::warn!("Failed to sync product update to Polar: {}", e);
        }
    }

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

    // Delete image from storage if exists
    if let Some(image_path) = &product.image_path {
        let _ = state.storage.delete(image_path).await;
    }

    // Archive in Polar if linked
    if let Some(polar_product_id) = &product.polar_product_id {
        if let Err(e) = state.polar.archive_product(polar_product_id).await {
            tracing::warn!("Failed to archive product in Polar: {}", e);
        }
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

        let content_type = field
            .content_type()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "image/jpeg".to_string());

        // Validate content type
        if !content_type.starts_with("image/") {
            return Err(AppError::BadRequest("Only image files are allowed".to_string()));
        }

        let data = field.bytes().await.map_err(|e| {
            AppError::BadRequest(format!("Failed to read upload: {}", e))
        })?;

        // Upload to storage (R2/local)
        let path = state
            .storage
            .upload(&filename, &data)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        // Sync image to Polar if product is linked
        if let Some(polar_product_id) = &product.polar_product_id {
            if let Err(e) = state
                .polar
                .upload_product_image(polar_product_id, &filename, &content_type, &data)
                .await
            {
                tracing::warn!("Failed to sync image to Polar: {}", e);
            }
        }

        // Update product with new image path
        let updated = Product::set_image(&conn, &id, &path).await?;

        return Ok(Json(AdminProductResponse::from_product(updated, &state)));
    }

    Err(AppError::BadRequest("No file uploaded".to_string()))
}
