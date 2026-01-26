use axum::{
    extract::{Multipart, Path, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::models::{CreateProduct, Product, ProductImage, ProductNotification, ProductStyle, UpdateProduct};
use crate::routes::AppState;

/// Sanitize a style name for use in folder paths
fn sanitize_style_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else if c == ' ' {
                '-'
            } else {
                '_'
            }
        })
        .collect()
}

#[derive(Serialize)]
pub struct ImageResponse {
    pub id: String,
    pub image_path: String,
    pub image_url: String,
    pub sort_order: i32,
}

#[derive(Serialize)]
pub struct AdminStyleResponse {
    pub id: String,
    pub name: String,
    pub stock_quantity: i64,
    pub image_id: Option<String>,
    pub sort_order: i64,
}

#[derive(Serialize)]
pub struct AdminProductResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub price_cents: i32,
    pub price: f64,
    pub images: Vec<ImageResponse>,
    pub styles: Vec<AdminStyleResponse>,
    pub stock_quantity: i32,
    pub is_active: bool,
    pub polar_product_id: Option<String>,
    pub polar_price_id: Option<String>,
    pub created_ts: i64,
    pub updated_ts: i64,
}

impl AdminProductResponse {
    fn from_product(
        product: Product,
        images: Vec<ProductImage>,
        styles: Vec<ProductStyle>,
        state: &AppState,
    ) -> Self {
        let image_responses: Vec<ImageResponse> = images
            .into_iter()
            .map(|img| {
                let image_url = if img.image_path.starts_with("http") {
                    img.image_path.clone()
                } else {
                    state.storage.public_url(&img.image_path)
                };
                ImageResponse {
                    id: img.id,
                    image_path: img.image_path,
                    image_url,
                    sort_order: img.sort_order,
                }
            })
            .collect();

        let style_responses: Vec<AdminStyleResponse> = styles
            .into_iter()
            .map(|style| AdminStyleResponse {
                id: style.id,
                name: style.name,
                stock_quantity: style.stock_quantity,
                image_id: style.image_id,
                sort_order: style.sort_order,
            })
            .collect();

        Self {
            id: product.id,
            name: product.name,
            description: product.description,
            price_cents: product.price_cents,
            price: product.price_cents as f64 / 100.0,
            images: image_responses,
            styles: style_responses,
            stock_quantity: product.stock_quantity,
            is_active: product.is_active,
            polar_product_id: product.polar_product_id,
            polar_price_id: product.polar_price_id,
            created_ts: product.created_ts,
            updated_ts: product.updated_ts,
        }
    }
}

#[derive(Deserialize)]
pub struct ReorderImagesRequest {
    pub image_ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct CreateStyleRequest {
    pub name: String,
    pub stock_quantity: i64,
    pub image_id: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateStyleRequest {
    pub name: String,
    pub stock_quantity: i64,
    pub image_id: Option<String>,
}

#[derive(Deserialize)]
pub struct ReorderStylesRequest {
    pub style_ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct BatchProductUpdate {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub price_cents: i32,
    pub stock_quantity: i32,
    pub is_active: bool,
    pub was_out_of_stock: bool,
    pub is_new: bool,
}

#[derive(Deserialize)]
pub struct BatchUpdateRequest {
    pub updates: Vec<BatchProductUpdate>,
    pub send_emails: bool,
}

#[derive(Serialize)]
pub struct BatchUpdateResponse {
    pub success: bool,
    pub updated_count: usize,
    pub emails_sent: usize,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/products", get(list_products))
        .route("/products", post(create_product))
        .route("/products-batch", put(batch_update_products))
        .route("/products/{id}", get(get_product))
        .route("/products/{id}", put(update_product))
        .route("/products/{id}", delete(delete_product))
        .route("/products/{id}/images", post(upload_image))
        .route("/products/{id}/images/reorder", put(reorder_images))
        .route("/products/{id}/images/{image_id}", delete(delete_image))
        .route("/products/{id}/sync-polar", post(sync_to_polar))
        // Style routes
        .route("/products/{id}/styles", post(create_style))
        .route("/products/{id}/styles/reorder", put(reorder_styles))
        .route("/products/{id}/styles/{style_id}", put(update_style))
        .route("/products/{id}/styles/{style_id}", delete(delete_style))
}

async fn list_products(State(state): State<AppState>) -> AppResult<Json<Vec<AdminProductResponse>>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let products = Product::list_all(&conn).await?;

    let mut responses = Vec::new();
    for product in products {
        let images = ProductImage::list_by_product(&conn, &product.id).await?;
        let styles = ProductStyle::get_by_product(&conn, &product.id).await?;
        responses.push(AdminProductResponse::from_product(product, images, styles, &state));
    }

    Ok(Json(responses))
}

async fn batch_update_products(
    State(state): State<AppState>,
    Json(payload): Json<BatchUpdateRequest>,
) -> AppResult<Json<BatchUpdateResponse>> {
    tracing::info!("Batch update request: {} products, send_emails: {}", payload.updates.len(), payload.send_emails);

    let conn = state.db.connect().map_err(AppError::from)?;
    let mut updated_count = 0;
    let mut emails_sent = 0;

    // Collect products that need restock notifications
    let mut restocked_products: Vec<(Product, Option<String>)> = Vec::new();

    for update in &payload.updates {
        tracing::info!("Updating product: {} ({})", update.name, update.id);

        // Update the product
        let update_data = UpdateProduct {
            name: Some(update.name.clone()),
            description: update.description.clone(),
            price_cents: Some(update.price_cents),
            stock_quantity: Some(update.stock_quantity),
            is_active: Some(update.is_active),
            image_path: None,
            polar_price_id: None,
        };

        let product = match Product::update(&conn, &update.id, update_data).await {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Failed to update product {}: {:?}", update.id, e);
                return Err(e);
            }
        };
        updated_count += 1;

        // Check if this is a restock (was out of stock, now has stock)
        if payload.send_emails && update.was_out_of_stock && update.stock_quantity > 0 {
            // Get first image for email
            let images = ProductImage::list_by_product(&conn, &product.id).await?;
            let image_url = images.first().map(|img| {
                if img.image_path.starts_with("http") {
                    img.image_path.clone()
                } else {
                    state.storage.public_url(&img.image_path)
                }
            });
            restocked_products.push((product, image_url));
        }
    }

    // Send restock notifications
    if payload.send_emails && !restocked_products.is_empty() {
        if let Some(ref resend) = state.resend {
            for (product, image_url) in &restocked_products {
                // Get all pending notifications for this product
                let notifications = ProductNotification::get_pending_for_product(&conn, &product.id).await?;

                for notification in &notifications {
                    if let Err(e) = resend
                        .send_product_restock_alert(&notification.email, product, image_url.as_deref())
                        .await
                    {
                        tracing::error!("Failed to send restock alert to {}: {}", notification.email, e);
                    } else {
                        emails_sent += 1;
                    }
                }

                // Mark all as notified
                ProductNotification::mark_all_notified_for_product(&conn, &product.id).await?;
            }
        }
    }

    Ok(Json(BatchUpdateResponse {
        success: true,
        updated_count,
        emails_sent,
    }))
}

async fn get_product(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<AdminProductResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let product = Product::find_by_id(&conn, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    let images = ProductImage::list_by_product(&conn, &id).await?;
    let styles = ProductStyle::get_by_product(&conn, &id).await?;

    Ok(Json(AdminProductResponse::from_product(product, images, styles, &state)))
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

    Ok(Json(AdminProductResponse::from_product(product, vec![], vec![], &state)))
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

    // Check if this is a restock (was 0, now > 0)
    let was_out_of_stock = current.stock_quantity == 0;
    let new_stock = payload.stock_quantity;

    // Extract values for Polar sync
    let name_update = payload.name.clone();
    let desc_update = payload.description.clone();

    let product = Product::update(&conn, &id, payload).await?;

    // Send restock notifications if product was restocked
    if was_out_of_stock && new_stock.map(|s| s > 0).unwrap_or(false) {
        let notifications = ProductNotification::get_pending_for_product(&conn, &id).await?;

        if !notifications.is_empty() {
            // Get first image for email
            let images = ProductImage::list_by_product(&conn, &id).await?;
            let image_url = images.first().map(|img| {
                if img.image_path.starts_with("http") {
                    img.image_path.clone()
                } else {
                    state.storage.public_url(&img.image_path)
                }
            });

            // Send notifications
            if let Some(ref resend) = state.resend {
                let mut sent_count = 0;
                for notification in &notifications {
                    if let Err(e) = resend
                        .send_product_restock_alert(&notification.email, &product, image_url.as_deref())
                        .await
                    {
                        tracing::error!("Failed to send restock alert to {}: {}", notification.email, e);
                    } else {
                        sent_count += 1;
                    }
                }
                tracing::info!("Sent {} restock notifications for product {}", sent_count, product.name);
            }

            // Mark all as notified
            ProductNotification::mark_all_notified_for_product(&conn, &id).await?;
        }
    }

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

    let images = ProductImage::list_by_product(&conn, &id).await?;
    let styles = ProductStyle::get_by_product(&conn, &id).await?;

    Ok(Json(AdminProductResponse::from_product(product, images, styles, &state)))
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

    // Delete all images from storage
    let _ = state.storage.delete_folder(&id).await;

    // Delete image records from database
    ProductImage::delete_by_product(&conn, &id).await?;

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

    // Process upload - can upload multiple images at once
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::BadRequest(format!("Failed to process upload: {}", e))
    })? {
        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "image.jpg".to_string());

        // Determine content type from header or extension
        let extension = filename.rsplit('.').next().unwrap_or("").to_lowercase();
        let content_type = field
            .content_type()
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                // Fallback to extension-based content type
                match extension.as_str() {
                    "jpg" | "jpeg" => "image/jpeg".to_string(),
                    "png" => "image/png".to_string(),
                    "gif" => "image/gif".to_string(),
                    "webp" => "image/webp".to_string(),
                    "svg" => "image/svg+xml".to_string(),
                    _ => "application/octet-stream".to_string(),
                }
            });

        // Validate content type or extension
        let valid_extensions = ["jpg", "jpeg", "png", "gif", "webp", "svg"];
        if !content_type.starts_with("image/") && !valid_extensions.contains(&extension.as_str()) {
            return Err(AppError::BadRequest("Only image files are allowed".to_string()));
        }

        let data = field.bytes().await.map_err(|e| {
            AppError::BadRequest(format!("Failed to read upload: {}", e))
        })?;

        // Upload to storage in product folder
        let path = state
            .storage
            .upload_to_folder(&id, &filename, &data)
            .await
            .map_err(|e| AppError::Storage(e.to_string()))?;

        // Add to product_images table
        ProductImage::add(&conn, &id, &path).await?;
    }

    let images = ProductImage::list_by_product(&conn, &id).await?;
    let styles = ProductStyle::get_by_product(&conn, &id).await?;

    // Sync all images to Polar if product is linked
    if product.polar_product_id.is_some() {
        if let Err(e) = sync_product_images_to_polar(&state, &product, &images).await {
            tracing::warn!("Failed to sync images to Polar: {}", e);
        }
    }

    Ok(Json(AdminProductResponse::from_product(product, images, styles, &state)))
}

async fn reorder_images(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ReorderImagesRequest>,
) -> AppResult<Json<AdminProductResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    // Verify product exists
    let product = Product::find_by_id(&conn, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    // Reorder images
    ProductImage::reorder(&conn, &id, &payload.image_ids).await?;

    let images = ProductImage::list_by_product(&conn, &id).await?;
    let styles = ProductStyle::get_by_product(&conn, &id).await?;

    // Sync reordered images to Polar
    if product.polar_product_id.is_some() {
        if let Err(e) = sync_product_images_to_polar(&state, &product, &images).await {
            tracing::warn!("Failed to sync reordered images to Polar: {}", e);
        }
    }

    Ok(Json(AdminProductResponse::from_product(product, images, styles, &state)))
}

async fn delete_image(
    State(state): State<AppState>,
    Path((product_id, image_id)): Path<(String, String)>,
) -> AppResult<Json<AdminProductResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    // Verify product exists
    let product = Product::find_by_id(&conn, &product_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    // Get image to delete from storage
    if let Some(image) = ProductImage::find_by_id(&conn, &image_id).await? {
        // Delete from storage
        let _ = state.storage.delete(&image.image_path).await;
    }

    // Delete from database
    ProductImage::delete(&conn, &image_id).await?;

    let images = ProductImage::list_by_product(&conn, &product_id).await?;
    let styles = ProductStyle::get_by_product(&conn, &product_id).await?;

    // Sync updated images to Polar
    if product.polar_product_id.is_some() {
        if let Err(e) = sync_product_images_to_polar(&state, &product, &images).await {
            tracing::warn!("Failed to sync images after delete to Polar: {}", e);
        }
    }

    Ok(Json(AdminProductResponse::from_product(product, images, styles, &state)))
}

/// Helper to sync all product images to Polar
async fn sync_product_images_to_polar(
    state: &AppState,
    product: &Product,
    images: &[ProductImage],
) -> AppResult<()> {
    let polar_product_id = match &product.polar_product_id {
        Some(id) => id,
        None => return Ok(()),
    };

    if images.is_empty() {
        // Clear all media from Polar product
        state.polar.set_product_media(polar_product_id, vec![]).await?;
        return Ok(());
    }

    // Download and upload each image to Polar
    let client = reqwest::Client::new();
    let mut file_ids = Vec::new();

    for image in images {
        let image_url = state.storage.public_url(&image.image_path);

        // Download image
        let response = match client.get(&image_url).send().await {
            Ok(r) if r.status().is_success() => r,
            _ => continue,
        };

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/jpeg")
            .to_string();

        let data = match response.bytes().await {
            Ok(d) => d,
            Err(_) => continue,
        };

        let filename = image.image_path.rsplit('/').next().unwrap_or("image.jpg");

        match state.polar.upload_file(filename, &content_type, &data).await {
            Ok(file_id) => file_ids.push(file_id),
            Err(e) => tracing::warn!("Failed to upload {} to Polar: {}", filename, e),
        }
    }

    // Set all media on the product
    state.polar.set_product_media(polar_product_id, file_ids).await?;
    Ok(())
}

#[derive(Serialize)]
pub struct SyncResponse {
    pub success: bool,
    pub synced_count: usize,
    pub message: String,
}

async fn sync_to_polar(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<SyncResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    // Get product
    let product = Product::find_by_id(&conn, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    // Check if product has Polar ID
    let polar_product_id = product.polar_product_id.ok_or_else(|| {
        AppError::BadRequest("Product not linked to Polar".to_string())
    })?;

    // Get all images for product
    let images = ProductImage::list_by_product(&conn, &id).await?;

    if images.is_empty() {
        return Ok(Json(SyncResponse {
            success: true,
            synced_count: 0,
            message: "No images to sync".to_string(),
        }));
    }

    // Upload each image and collect file IDs
    let client = reqwest::Client::new();
    let mut file_ids = Vec::new();

    for image in &images {
        let image_url = state.storage.public_url(&image.image_path);

        // Download image
        let response = client.get(&image_url).send().await.map_err(|e| {
            AppError::ExternalService(format!("Failed to download image: {}", e))
        })?;

        if !response.status().is_success() {
            tracing::warn!("Failed to download image {}: {}", image_url, response.status());
            continue;
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/jpeg")
            .to_string();

        let data = response.bytes().await.map_err(|e| {
            AppError::ExternalService(format!("Failed to read image data: {}", e))
        })?;

        // Extract filename from path
        let filename = image.image_path.rsplit('/').next().unwrap_or("image.jpg");

        // Upload to Polar
        match state.polar.upload_file(filename, &content_type, &data).await {
            Ok(file_id) => {
                file_ids.push(file_id);
                tracing::info!("Uploaded {} to Polar", filename);
            }
            Err(e) => {
                tracing::warn!("Failed to upload {} to Polar: {}", filename, e);
            }
        }
    }

    if file_ids.is_empty() {
        return Ok(Json(SyncResponse {
            success: false,
            synced_count: 0,
            message: "Failed to upload any images".to_string(),
        }));
    }

    // Set all media on the product at once
    state.polar.set_product_media(&polar_product_id, file_ids.clone()).await?;

    Ok(Json(SyncResponse {
        success: true,
        synced_count: file_ids.len(),
        message: format!("Synced {} images to Polar", file_ids.len()),
    }))
}

// Style CRUD handlers

async fn create_style(
    State(state): State<AppState>,
    Path(product_id): Path<String>,
    Json(payload): Json<CreateStyleRequest>,
) -> AppResult<Json<AdminProductResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    // Verify product exists
    let product = Product::find_by_id(&conn, &product_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    // If an image is being linked, move it to the style folder
    if let Some(ref image_id) = payload.image_id {
        if let Some(image) = ProductImage::find_by_id(&conn, image_id).await? {
            let style_folder = format!("{}/{}", product_id, sanitize_style_name(&payload.name));
            match state.storage.move_object(&image.image_path, &style_folder).await {
                Ok(new_path) => {
                    ProductImage::update_path(&conn, image_id, &new_path).await?;
                }
                Err(e) => {
                    tracing::warn!("Failed to move image to style folder: {}", e);
                }
            }
        }
    }

    // Create the style
    ProductStyle::create(
        &conn,
        &product_id,
        &payload.name,
        payload.stock_quantity,
        payload.image_id.as_deref(),
    )
    .await?;

    let images = ProductImage::list_by_product(&conn, &product_id).await?;
    let styles = ProductStyle::get_by_product(&conn, &product_id).await?;

    Ok(Json(AdminProductResponse::from_product(product, images, styles, &state)))
}

async fn update_style(
    State(state): State<AppState>,
    Path((product_id, style_id)): Path<(String, String)>,
    Json(payload): Json<UpdateStyleRequest>,
) -> AppResult<Json<AdminProductResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    // Verify product exists
    let product = Product::find_by_id(&conn, &product_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    // Verify style exists and belongs to this product
    let style = ProductStyle::get_by_id(&conn, &style_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Style not found".to_string()))?;

    if style.product_id != product_id {
        return Err(AppError::BadRequest("Style does not belong to this product".to_string()));
    }

    // If an image is being linked (new or changed), move it to the style folder
    if let Some(ref image_id) = payload.image_id {
        // Only move if the image is different from the current one
        if style.image_id.as_ref() != Some(image_id) {
            if let Some(image) = ProductImage::find_by_id(&conn, image_id).await? {
                let style_folder = format!("{}/{}", product_id, sanitize_style_name(&payload.name));
                match state.storage.move_object(&image.image_path, &style_folder).await {
                    Ok(new_path) => {
                        ProductImage::update_path(&conn, image_id, &new_path).await?;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to move image to style folder: {}", e);
                    }
                }
            }
        } else if style.name != payload.name {
            // Image is the same but name changed - move image to new folder name
            if let Some(image) = ProductImage::find_by_id(&conn, image_id).await? {
                let style_folder = format!("{}/{}", product_id, sanitize_style_name(&payload.name));
                match state.storage.move_object(&image.image_path, &style_folder).await {
                    Ok(new_path) => {
                        ProductImage::update_path(&conn, image_id, &new_path).await?;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to move image to new style folder: {}", e);
                    }
                }
            }
        }
    }

    // Update the style
    ProductStyle::update(
        &conn,
        &style_id,
        &payload.name,
        payload.stock_quantity,
        payload.image_id.as_deref(),
    )
    .await?;

    let images = ProductImage::list_by_product(&conn, &product_id).await?;
    let styles = ProductStyle::get_by_product(&conn, &product_id).await?;

    Ok(Json(AdminProductResponse::from_product(product, images, styles, &state)))
}

async fn delete_style(
    State(state): State<AppState>,
    Path((product_id, style_id)): Path<(String, String)>,
) -> AppResult<Json<AdminProductResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    // Verify product exists
    let product = Product::find_by_id(&conn, &product_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    // Verify style exists and belongs to this product
    let style = ProductStyle::get_by_id(&conn, &style_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Style not found".to_string()))?;

    if style.product_id != product_id {
        return Err(AppError::BadRequest("Style does not belong to this product".to_string()));
    }

    // Delete the style
    ProductStyle::delete(&conn, &style_id).await?;

    let images = ProductImage::list_by_product(&conn, &product_id).await?;
    let styles = ProductStyle::get_by_product(&conn, &product_id).await?;

    Ok(Json(AdminProductResponse::from_product(product, images, styles, &state)))
}

async fn reorder_styles(
    State(state): State<AppState>,
    Path(product_id): Path<String>,
    Json(payload): Json<ReorderStylesRequest>,
) -> AppResult<Json<AdminProductResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    // Verify product exists
    let product = Product::find_by_id(&conn, &product_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    // Reorder styles
    ProductStyle::reorder(&conn, &product_id, &payload.style_ids).await?;

    let images = ProductImage::list_by_product(&conn, &product_id).await?;
    let styles = ProductStyle::get_by_product(&conn, &product_id).await?;

    Ok(Json(AdminProductResponse::from_product(product, images, styles, &state)))
}
