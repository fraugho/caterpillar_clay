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
    pub easypost_tracker_id: Option<String>,
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

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/orders", get(list_orders))
        .route("/orders/{id}", get(get_order))
        .route("/orders/{id}/status", put(update_status))
        .route("/orders/{id}/tracking", post(add_tracking))
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
            easypost_tracker_id: order.easypost_tracker_id.clone(),
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
        easypost_tracker_id: order.easypost_tracker_id.clone(),
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
        easypost_tracker_id: order.easypost_tracker_id.clone(),
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

    // Create EasyPost tracker
    let tracker = state
        .easypost
        .create_tracker(&payload.tracking_number, payload.carrier.as_deref())
        .await?;

    // Update order with tracking info
    let order = Order::set_tracking(&conn, &id, &payload.tracking_number, Some(&tracker.id))
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
        easypost_tracker_id: order.easypost_tracker_id.clone(),
        items,
        created_ts: order.created_ts,
        updated_ts: order.updated_ts,
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
