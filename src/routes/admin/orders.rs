use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use libsql::Connection;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::models::{Order, OrderStatus, Product, ShippingAddress, User};
use crate::routes::AppState;

#[derive(Serialize)]
pub struct AdminOrderResponse {
    pub id: String,
    pub user: Option<OrderUserInfo>,
    pub status: String,
    pub total_cents: i32,
    pub total: f64,
    pub shipping_address: Option<ShippingAddress>,
    pub tracking_number: Option<String>,
    pub shippo_tracker_id: Option<String>,
    pub stripe_payment_intent_id: Option<String>,
    pub items: Vec<AdminOrderItemResponse>,
    pub created_ts: i64,
    pub updated_ts: i64,
}

#[derive(Serialize)]
pub struct OrderUserInfo {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
}

#[derive(Serialize)]
pub struct AdminOrderItemResponse {
    pub product_id: String,
    pub product_name: String,
    pub quantity: i32,
    pub price_cents: i32,
}

#[derive(Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
}

#[derive(Deserialize)]
pub struct AddTrackingRequest {
    pub tracking_number: String,
    pub carrier: Option<String>,
}

#[derive(Deserialize)]
pub struct RefundRequest {
    pub reason: Option<String>,
}

#[derive(Serialize)]
pub struct RefundResponse {
    pub refund_id: String,
    pub status: String,
    pub amount_cents: i64,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/orders", get(list_orders))
        .route("/orders/{id}", get(get_order))
        .route("/orders/{id}/status", put(update_status))
        .route("/orders/{id}/tracking", post(add_tracking))
        .route("/orders/{id}/refund", post(refund_order))
}

async fn list_orders(State(state): State<AppState>) -> AppResult<Json<Vec<AdminOrderResponse>>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let orders = Order::list_all(&conn).await?;

    let mut responses = Vec::new();
    for order in orders {
        let user_info = if let Some(ref user_id) = order.user_id {
            User::find_by_id(&conn, user_id)
                .await?
                .map(|u| OrderUserInfo {
                    id: u.id,
                    email: u.email,
                    name: u.name,
                })
        } else {
            None
        };

        let items = build_order_items(&conn, &order.id).await?;

        responses.push(AdminOrderResponse {
            id: order.id.clone(),
            user: user_info,
            status: order.status.clone(),
            total_cents: order.total_cents,
            total: order.total_cents as f64 / 100.0,
            shipping_address: order.get_shipping_address(),
            tracking_number: order.tracking_number.clone(),
            shippo_tracker_id: order.shippo_tracker_id.clone(),
            stripe_payment_intent_id: order.stripe_payment_intent_id.clone(),
            items,
            created_ts: order.created_ts,
            updated_ts: order.updated_ts,
        });
    }

    Ok(Json(responses))
}

async fn get_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<AdminOrderResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let order = Order::find_by_id(&conn, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Order not found".to_string()))?;

    let user_info = if let Some(ref user_id) = order.user_id {
        User::find_by_id(&conn, user_id)
            .await?
            .map(|u| OrderUserInfo {
                id: u.id,
                email: u.email,
                name: u.name,
            })
    } else {
        None
    };

    let items = build_order_items(&conn, &order.id).await?;

    Ok(Json(AdminOrderResponse {
        id: order.id.clone(),
        user: user_info,
        status: order.status.clone(),
        total_cents: order.total_cents,
        total: order.total_cents as f64 / 100.0,
        shipping_address: order.get_shipping_address(),
        tracking_number: order.tracking_number.clone(),
        shippo_tracker_id: order.shippo_tracker_id.clone(),
        stripe_payment_intent_id: order.stripe_payment_intent_id.clone(),
        items,
        created_ts: order.created_ts,
        updated_ts: order.updated_ts,
    }))
}

async fn update_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateStatusRequest>,
) -> AppResult<Json<AdminOrderResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    let status = OrderStatus::from_str(&payload.status)
        .ok_or_else(|| AppError::BadRequest("Invalid status".to_string()))?;

    let order = Order::update_status(&conn, &id, status).await?;

    let user_info = if let Some(ref user_id) = order.user_id {
        User::find_by_id(&conn, user_id)
            .await?
            .map(|u| OrderUserInfo {
                id: u.id,
                email: u.email,
                name: u.name,
            })
    } else {
        None
    };

    let items = build_order_items(&conn, &order.id).await?;

    Ok(Json(AdminOrderResponse {
        id: order.id.clone(),
        user: user_info,
        status: order.status.clone(),
        total_cents: order.total_cents,
        total: order.total_cents as f64 / 100.0,
        shipping_address: order.get_shipping_address(),
        tracking_number: order.tracking_number.clone(),
        shippo_tracker_id: order.shippo_tracker_id.clone(),
        stripe_payment_intent_id: order.stripe_payment_intent_id.clone(),
        items,
        created_ts: order.created_ts,
        updated_ts: order.updated_ts,
    }))
}

async fn add_tracking(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<AddTrackingRequest>,
) -> AppResult<Json<AdminOrderResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    // Verify order exists
    Order::find_by_id(&conn, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Order not found".to_string()))?;

    // Register tracking with Shippo
    let carrier = payload.carrier.as_deref().unwrap_or("usps");
    let tracking = state
        .shippo
        .register_tracking(&payload.tracking_number, carrier)
        .await?;

    // Update order with tracking info
    let order = Order::set_tracking(&conn, &id, &payload.tracking_number, Some(&tracking.tracking_number))
        .await?;

    // Send shipping notification email
    if let Some(ref email_service) = state.email {
        if let Some(ref user_id) = order.user_id {
            if let Ok(Some(user)) = User::find_by_id(&conn, user_id).await {
                let name = user.name.as_deref().unwrap_or("Customer");
                let _ = email_service
                    .send_order_shipped(&user.email, &order, name, &payload.tracking_number)
                    .await;
            }
        }
    }

    let user_info = if let Some(ref user_id) = order.user_id {
        User::find_by_id(&conn, user_id)
            .await?
            .map(|u| OrderUserInfo {
                id: u.id,
                email: u.email,
                name: u.name,
            })
    } else {
        None
    };

    let items = build_order_items(&conn, &order.id).await?;

    Ok(Json(AdminOrderResponse {
        id: order.id.clone(),
        user: user_info,
        status: order.status.clone(),
        total_cents: order.total_cents,
        total: order.total_cents as f64 / 100.0,
        shipping_address: order.get_shipping_address(),
        tracking_number: order.tracking_number.clone(),
        shippo_tracker_id: order.shippo_tracker_id.clone(),
        stripe_payment_intent_id: order.stripe_payment_intent_id.clone(),
        items,
        created_ts: order.created_ts,
        updated_ts: order.updated_ts,
    }))
}

async fn refund_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<RefundRequest>,
) -> AppResult<Json<RefundResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    // Get the order
    let order = Order::find_by_id(&conn, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Order not found".to_string()))?;

    // Check if order has a payment intent ID
    let payment_intent_id = order.stripe_payment_intent_id.as_ref()
        .ok_or_else(|| AppError::BadRequest("Order has no payment intent - cannot refund".to_string()))?;

    // Check if order is in a refundable state
    let status = OrderStatus::from_str(&order.status);
    match status {
        Some(OrderStatus::Paid) | Some(OrderStatus::Processing) | Some(OrderStatus::Shipped) | Some(OrderStatus::Delivered) => {
            // These statuses are refundable
        }
        Some(OrderStatus::Refunded) => {
            return Err(AppError::BadRequest("Order has already been refunded".to_string()));
        }
        Some(OrderStatus::Pending) => {
            return Err(AppError::BadRequest("Order is still pending - cannot refund unpaid order".to_string()));
        }
        Some(OrderStatus::Cancelled) => {
            return Err(AppError::BadRequest("Order was cancelled - cannot refund".to_string()));
        }
        None => {
            return Err(AppError::BadRequest("Unknown order status".to_string()));
        }
    }

    // Create refund via Stripe (full refund)
    let refund = state.stripe.create_refund(
        payment_intent_id,
        None, // Full refund
        payload.reason.as_deref(),
    ).await?;

    tracing::info!("Created refund {} for order {} (amount: {} cents)", refund.id, id, refund.amount);

    // Note: The webhook will handle updating order status and restoring stock

    Ok(Json(RefundResponse {
        refund_id: refund.id,
        status: refund.status,
        amount_cents: refund.amount,
    }))
}

async fn build_order_items(
    conn: &Connection,
    order_id: &str,
) -> AppResult<Vec<AdminOrderItemResponse>> {
    let items = Order::get_items(conn, order_id).await?;
    let mut responses = Vec::new();

    for item in items {
        let product_name = match Product::find_by_id(conn, &item.product_id).await? {
            Some(p) => p.name,
            None => "Unknown Product".to_string(),
        };

        responses.push(AdminOrderItemResponse {
            product_id: item.product_id,
            product_name,
            quantity: item.quantity,
            price_cents: item.price_cents,
        });
    }

    Ok(responses)
}
