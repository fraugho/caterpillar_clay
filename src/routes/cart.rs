use axum::{
    extract::{Extension, State},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::middleware::AuthUser;
use crate::models::{CreateOrder, CreateOrderItem, Order, Product, ShippingAddress};
use crate::routes::AppState;

#[derive(Deserialize)]
pub struct CartItem {
    pub product_id: String,
    pub quantity: i32,
}

#[derive(Deserialize)]
pub struct CheckoutRequest {
    pub items: Vec<CartItem>,
    pub shipping_address: ShippingAddress,
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

    // Create order in pending state
    let order = Order::create(
        &conn,
        CreateOrder {
            user_id: Some(user.id.clone()),
            total_cents,
            shipping_address: payload.shipping_address,
            polar_checkout_id: None,
            items: order_items,
        },
    )
    .await?;

    // Create Polar checkout session
    let success_url = format!("{}/orders/{}?success=true", state.config.base_url, order.id);

    // Get the first product's polar_price_id for checkout
    // Note: For multi-item orders, we use the first product's price as the checkout item
    // The full order details are tracked in our database
    let first_item = payload.items.first().unwrap();
    let first_product = Product::find_by_id(&conn, &first_item.product_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;

    let polar_price_id = first_product.polar_price_id.ok_or_else(|| {
        AppError::BadRequest("Product not configured for checkout. Please contact support.".to_string())
    })?;

    let checkout = state
        .polar
        .create_checkout(
            &polar_price_id,
            &success_url,
            Some(&user.email),
            order.uuid().unwrap_or_default(),
            user.id.parse().ok(),
        )
        .await?;

    Ok(Json(CheckoutResponse {
        checkout_url: checkout.url,
        order_id: order.id,
    }))
}
