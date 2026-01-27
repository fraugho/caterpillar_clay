use axum::{
    extract::{Extension, State},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::middleware::AuthUser;
use crate::models::{CreateOrder, CreateOrderItem, Order, Product, ProductImage, ShippingAddress};
use crate::routes::AppState;
use crate::services::stripe::CheckoutItem;

#[derive(Deserialize)]
pub struct CartItem {
    pub product_id: String,
    pub quantity: i32,
}

#[derive(Deserialize)]
pub struct CheckoutRequest {
    pub items: Vec<CartItem>,
    pub shipping_address: ShippingAddress,
    // Shipping selection
    pub shipping_rate_id: Option<String>,
    pub shipping_cents: Option<i32>,
    pub shipping_carrier: Option<String>,
    pub shipping_service: Option<String>,
    pub estimated_delivery_days: Option<i32>,
}

#[derive(Serialize)]
pub struct CheckoutResponse {
    pub checkout_url: String,
    pub order_id: String,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/checkout", post(create_checkout))
}

async fn create_checkout(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Json(payload): Json<CheckoutRequest>,
) -> AppResult<Json<CheckoutResponse>> {
    if payload.items.is_empty() {
        return Err(AppError::BadRequest("Cart is empty".to_string()));
    }

    let conn = state.db.connect().map_err(AppError::from)?;

    // Calculate total and validate products
    let mut total_cents = 0i32;
    let mut order_items: Vec<CreateOrderItem> = Vec::new();

    for item in &payload.items {
        let product = Product::find_by_id(&conn, &item.product_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Product {} not found", item.product_id)))?;

        if !product.is_active {
            return Err(AppError::BadRequest(format!(
                "Product {} is not available",
                product.name
            )));
        }

        if product.stock_quantity < item.quantity {
            return Err(AppError::BadRequest(format!(
                "Insufficient stock for {}",
                product.name
            )));
        }

        let item_total = product.price_cents * item.quantity;
        total_cents += item_total;

        order_items.push(CreateOrderItem {
            product_id: product.id,
            quantity: item.quantity,
            price_cents: product.price_cents,
        });
    }

    // Add shipping cost to total
    let shipping_cents = payload.shipping_cents.unwrap_or(0);
    total_cents += shipping_cents;

    // Build checkout items with product details
    let mut checkout_items: Vec<CheckoutItem> = Vec::new();
    for item in &payload.items {
        let product = Product::find_by_id(&conn, &item.product_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Product {} not found", item.product_id)))?;

        // Get product images for checkout display
        let images = ProductImage::list_by_product(&conn, &item.product_id).await?;
        let image_urls: Vec<String> = images
            .iter()
            .take(1) // Stripe checkout shows one image per line item
            .map(|img| {
                if img.image_path.starts_with("http") {
                    img.image_path.clone()
                } else {
                    state.storage.public_url(&img.image_path)
                }
            })
            .collect();

        checkout_items.push(CheckoutItem {
            name: product.name.clone(),
            description: product.description.clone(),
            images: if image_urls.is_empty() { None } else { Some(image_urls) },
            price_cents: product.price_cents as i64,
            quantity: item.quantity,
        });
    }

    // Add shipping as a line item if selected
    if shipping_cents > 0 {
        let shipping_description = payload.estimated_delivery_days
            .map(|d| format!("Estimated {} days", d));
        let shipping_name = match (&payload.shipping_carrier, &payload.shipping_service) {
            (Some(carrier), Some(service)) => format!("{} - {}", carrier, service),
            (Some(carrier), None) => format!("{} Shipping", carrier),
            _ => "Shipping".to_string(),
        };

        checkout_items.push(CheckoutItem {
            name: shipping_name,
            description: shipping_description,
            images: None,
            price_cents: shipping_cents as i64,
            quantity: 1,
        });
    }

    // Create order in pending state (without session ID initially)
    let order = Order::create(
        &conn,
        CreateOrder {
            user_id: Some(user.id.clone()),
            total_cents,
            shipping_address: payload.shipping_address,
            stripe_session_id: None,
            items: order_items,
            // Shipping details
            shipping_cents: Some(shipping_cents),
            shipping_carrier: payload.shipping_carrier,
            shipping_service: payload.shipping_service,
            estimated_delivery_days: payload.estimated_delivery_days,
        },
    )
    .await?;

    // Create Stripe checkout session
    let success_url = format!("{}/orders/{}?success=true", state.config.base_url, order.id);
    let cancel_url = format!("{}/cart?cancelled=true", state.config.base_url);

    let checkout = state
        .stripe
        .create_checkout_session(
            checkout_items,
            &success_url,
            &cancel_url,
            Some(&user.email),
            &order.id,
        )
        .await?;

    // Update order with Stripe session ID
    Order::set_stripe_session(&conn, &order.id, &checkout.id).await?;

    Ok(Json(CheckoutResponse {
        checkout_url: checkout.url,
        order_id: order.id,
    }))
}
