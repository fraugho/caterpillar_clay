use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::models::{NewsletterSubscriber, Product, ProductImage};
use crate::routes::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/newsletter/subscribers", get(get_subscriber_count))
        .route("/newsletter/notify/new/{product_id}", post(notify_new_product))
        .route("/newsletter/notify/restock/{product_id}", post(notify_back_in_stock))
        .route("/newsletter/notify-batch/{notify_type}", post(notify_batch))
}

#[derive(Deserialize)]
pub struct BatchNotifyRequest {
    pub product_ids: Vec<String>,
}

#[derive(Serialize)]
pub struct SubscriberCountResponse {
    pub count: i64,
}

async fn get_subscriber_count(
    State(state): State<AppState>,
) -> AppResult<Json<SubscriberCountResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let count = NewsletterSubscriber::count(&conn).await?;
    Ok(Json(SubscriberCountResponse { count }))
}

#[derive(Serialize)]
pub struct NotifyResponse {
    pub success: bool,
    pub sent_count: usize,
    pub total_subscribers: usize,
}

async fn notify_new_product(
    State(state): State<AppState>,
    Path(product_id): Path<String>,
) -> AppResult<Json<NotifyResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    // Get the product
    let product = Product::find_by_id(&conn, &product_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    // Get all subscribers
    let subscribers = NewsletterSubscriber::get_all(&conn).await?;
    let total_subscribers = subscribers.len();

    if total_subscribers == 0 {
        return Ok(Json(NotifyResponse {
            success: true,
            sent_count: 0,
            total_subscribers: 0,
        }));
    }

    // Check if Resend is configured
    let resend = state.resend.as_ref().ok_or_else(|| {
        AppError::Internal("Newsletter service not configured. Set RESEND_API_KEY.".to_string())
    })?;

    // Get product's first image URL if available
    let images = ProductImage::list_by_product(&conn, &product_id).await?;
    let first_image_url = images.first().map(|img| state.storage.public_url(&img.image_path));

    // Prepare subscriber list
    let subscriber_list: Vec<(String, String)> = subscribers
        .into_iter()
        .map(|s| (s.email, s.unsubscribe_token))
        .collect();

    // Send notifications
    let sent_count = resend
        .send_batch_new_product_notification(&subscriber_list, &product, first_image_url.as_deref())
        .await?;

    Ok(Json(NotifyResponse {
        success: true,
        sent_count,
        total_subscribers,
    }))
}

async fn notify_back_in_stock(
    State(state): State<AppState>,
    Path(product_id): Path<String>,
) -> AppResult<Json<NotifyResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    // Get the product
    let product = Product::find_by_id(&conn, &product_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    // Get all subscribers
    let subscribers = NewsletterSubscriber::get_all(&conn).await?;
    let total_subscribers = subscribers.len();

    if total_subscribers == 0 {
        return Ok(Json(NotifyResponse {
            success: true,
            sent_count: 0,
            total_subscribers: 0,
        }));
    }

    // Check if Resend is configured
    let resend = state.resend.as_ref().ok_or_else(|| {
        AppError::Internal("Newsletter service not configured. Set RESEND_API_KEY.".to_string())
    })?;

    // Get product's first image URL if available
    let images = ProductImage::list_by_product(&conn, &product_id).await?;
    let first_image_url = images.first().map(|img| state.storage.public_url(&img.image_path));

    // Prepare subscriber list
    let subscriber_list: Vec<(String, String)> = subscribers
        .into_iter()
        .map(|s| (s.email, s.unsubscribe_token))
        .collect();

    // Send back in stock notifications
    let sent_count = resend
        .send_batch_back_in_stock_notification(&subscriber_list, &product, first_image_url.as_deref())
        .await?;

    Ok(Json(NotifyResponse {
        success: true,
        sent_count,
        total_subscribers,
    }))
}

async fn notify_batch(
    State(state): State<AppState>,
    Path(notify_type): Path<String>,
    Json(payload): Json<BatchNotifyRequest>,
) -> AppResult<Json<NotifyResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    if payload.product_ids.is_empty() {
        return Ok(Json(NotifyResponse {
            success: true,
            sent_count: 0,
            total_subscribers: 0,
        }));
    }

    // Get all products with their images
    let mut products_with_images: Vec<(Product, Option<String>)> = Vec::new();
    for product_id in &payload.product_ids {
        if let Some(product) = Product::find_by_id(&conn, product_id).await? {
            let images = ProductImage::list_by_product(&conn, product_id).await?;
            let first_image_url = images.first().map(|img| state.storage.public_url(&img.image_path));
            products_with_images.push((product, first_image_url));
        }
    }

    if products_with_images.is_empty() {
        return Err(AppError::NotFound("No valid products found".to_string()));
    }

    // Get all subscribers
    let subscribers = NewsletterSubscriber::get_all(&conn).await?;
    let total_subscribers = subscribers.len();

    if total_subscribers == 0 {
        return Ok(Json(NotifyResponse {
            success: true,
            sent_count: 0,
            total_subscribers: 0,
        }));
    }

    // Check if Resend is configured
    let resend = state.resend.as_ref().ok_or_else(|| {
        AppError::Internal("Newsletter service not configured. Set RESEND_API_KEY.".to_string())
    })?;

    // Prepare subscriber list
    let subscriber_list: Vec<(String, String)> = subscribers
        .into_iter()
        .map(|s| (s.email, s.unsubscribe_token))
        .collect();

    // Send batch notification based on type
    let sent_count = match notify_type.as_str() {
        "new" => resend.send_batch_multi_product_new(&subscriber_list, &products_with_images).await?,
        "restock" => resend.send_batch_multi_product_restock(&subscriber_list, &products_with_images).await?,
        _ => return Err(AppError::BadRequest("Invalid notify type".to_string())),
    };

    Ok(Json(NotifyResponse {
        success: true,
        sent_count,
        total_subscribers,
    }))
}
