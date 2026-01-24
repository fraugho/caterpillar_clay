use axum::{
    extract::{Extension, Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::middleware::AuthUser;
use crate::models::{Order, OrderItem, Product, ShippingAddress};
use crate::routes::AppState;

#[derive(Serialize)]
pub struct OrderResponse {
    pub id: String,
    pub status: String,
    pub total_cents: i32,
    pub total: f64,
    pub shipping_address: Option<ShippingAddress>,
    pub tracking_number: Option<String>,
    pub items: Vec<OrderItemResponse>,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct OrderItemResponse {
    pub product_id: String,
    pub product_name: String,
    pub quantity: i32,
    pub price_cents: i32,
    pub price: f64,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/orders", get(list_orders))
        .route("/orders/{id}", get(get_order))
}

async fn list_orders(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
) -> AppResult<Json<Vec<OrderResponse>>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let orders = Order::list_by_user(&conn, &user.id).await?;

    let mut responses = Vec::new();
    for order in orders {
        let items = Order::get_items(&conn, &order.id).await?;
        let item_responses = build_item_responses(&conn, items).await?;

        responses.push(OrderResponse {
            id: order.id.clone(),
            status: order.status.clone(),
            total_cents: order.total_cents,
            total: order.total_cents as f64 / 100.0,
            shipping_address: order.get_shipping_address(),
            tracking_number: order.tracking_number.clone(),
            items: item_responses,
            created_at: order.created_at,
        });
    }

    Ok(Json(responses))
}

async fn get_order(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Path(id): Path<String>,
) -> AppResult<Json<OrderResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let order = Order::find_by_id(&conn, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Order not found".to_string()))?;

    // Verify ownership
    if order.user_id.as_ref() != Some(&user.id) {
        return Err(AppError::Forbidden("Access denied".to_string()));
    }

    let items = Order::get_items(&conn, &order.id).await?;
    let item_responses = build_item_responses(&conn, items).await?;

    Ok(Json(OrderResponse {
        id: order.id.clone(),
        status: order.status.clone(),
        total_cents: order.total_cents,
        total: order.total_cents as f64 / 100.0,
        shipping_address: order.get_shipping_address(),
        tracking_number: order.tracking_number.clone(),
        items: item_responses,
        created_at: order.created_at,
    }))
}

async fn build_item_responses(
    conn: &libsql::Connection,
    items: Vec<OrderItem>,
) -> AppResult<Vec<OrderItemResponse>> {
    let mut responses = Vec::new();

    for item in items {
        let product_name = match Product::find_by_id(conn, &item.product_id).await? {
            Some(p) => p.name,
            None => "Unknown Product".to_string(),
        };

        responses.push(OrderItemResponse {
            product_id: item.product_id,
            product_name,
            quantity: item.quantity,
            price_cents: item.price_cents,
            price: item.price_cents as f64 / 100.0,
        });
    }

    Ok(responses)
}
