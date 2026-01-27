use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use libsql::Connection;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::models::{Order, OrderStatus, Product, Setting, ShippingAddress, User};
use crate::routes::AppState;
use crate::services::shippo::{ShippoAddress, ShippoParcel};

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
    pub label_url: Option<String>,
    pub shipping_carrier: Option<String>,
    pub shipping_service: Option<String>,
    pub shipping_cents: i32,
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

#[derive(Serialize)]
pub struct ShippingRateOption {
    pub rate_id: String,
    pub carrier: String,
    pub service: String,
    pub price_cents: i32,
    pub estimated_days: Option<i32>,
}

#[derive(Deserialize)]
pub struct PurchaseLabelRequest {
    pub rate_id: String,
}

#[derive(Serialize)]
pub struct PurchaseLabelResponse {
    pub tracking_number: String,
    pub label_url: String,
    pub carrier: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/orders", get(list_orders))
        .route("/orders/{id}", get(get_order))
        .route("/orders/{id}/status", put(update_status))
        .route("/orders/{id}/tracking", post(add_tracking))
        .route("/orders/{id}/refund", post(refund_order))
        .route("/orders/{id}/shipping-rates", get(get_shipping_rates))
        .route("/orders/{id}/buy-label", post(buy_label))
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
            label_url: order.label_url.clone(),
            shipping_carrier: order.shipping_carrier.clone(),
            shipping_service: order.shipping_service.clone(),
            shipping_cents: order.shipping_cents,
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
        label_url: order.label_url.clone(),
        shipping_carrier: order.shipping_carrier.clone(),
        shipping_service: order.shipping_service.clone(),
        shipping_cents: order.shipping_cents,
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
        label_url: order.label_url.clone(),
        shipping_carrier: order.shipping_carrier.clone(),
        shipping_service: order.shipping_service.clone(),
        shipping_cents: order.shipping_cents,
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
        label_url: order.label_url.clone(),
        shipping_carrier: order.shipping_carrier.clone(),
        shipping_service: order.shipping_service.clone(),
        shipping_cents: order.shipping_cents,
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

async fn get_shipping_rates(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<Vec<ShippingRateOption>>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    // Get order and its items
    let order = Order::find_by_id(&conn, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Order not found".to_string()))?;

    let shipping_address = order.get_shipping_address()
        .ok_or_else(|| AppError::BadRequest("Order has no shipping address".to_string()))?;

    // Get shop address (origin)
    let shop_address = Setting::get_shop_address(&conn)
        .await?
        .ok_or_else(|| AppError::BadRequest("Shop address not configured".to_string()))?;

    // Get unit system preference
    let unit_system = Setting::get_unit_system(&conn).await?;
    let (distance_unit, mass_unit) = if unit_system == "metric" {
        ("cm", "g")
    } else {
        ("in", "oz")
    };

    // Calculate parcel dimensions from order items
    let items = Order::get_items(&conn, &id).await?;
    let mut total_weight = 0.0f64;
    let mut max_length = 0.0f64;
    let mut max_width = 0.0f64;
    let mut total_height = 0.0f64;

    for item in &items {
        let product = Product::find_by_id(&conn, &item.product_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Product {} not found", item.product_id)))?;

        // Use product dimensions or defaults
        let weight = product.weight_grams.unwrap_or(500) as f64;
        let length = product.length_cm.unwrap_or(15.0);
        let width = product.width_cm.unwrap_or(15.0);
        let height = product.height_cm.unwrap_or(10.0);

        total_weight += weight * item.quantity as f64;
        max_length = max_length.max(length);
        max_width = max_width.max(width);
        total_height += height * item.quantity as f64;
    }

    // Convert units if US system
    let (final_weight, final_length, final_width, final_height) = if unit_system == "us" {
        (
            total_weight * 0.035274,
            max_length * 0.393701,
            max_width * 0.393701,
            total_height * 0.393701,
        )
    } else {
        (total_weight, max_length, max_width, total_height)
    };

    // Build addresses for Shippo
    let from_address = ShippoAddress {
        name: shop_address.name,
        street1: shop_address.street1,
        street2: shop_address.street2,
        city: shop_address.city,
        state: shop_address.state,
        zip: shop_address.zip,
        country: shop_address.country,
        phone: shop_address.phone,
    };

    let to_address = ShippoAddress {
        name: shipping_address.name,
        street1: shipping_address.street,
        street2: None,
        city: shipping_address.city,
        state: shipping_address.state,
        zip: shipping_address.zip,
        country: shipping_address.country,
        phone: None,
    };

    let parcel = ShippoParcel {
        length: final_length,
        width: final_width,
        height: final_height,
        distance_unit: distance_unit.to_string(),
        weight: final_weight,
        mass_unit: mass_unit.to_string(),
    };

    // Get rates from Shippo
    let shippo_rates = state.shippo.get_rates(from_address, to_address, vec![parcel]).await?;

    // Convert to response format
    let rates: Vec<ShippingRateOption> = shippo_rates
        .into_iter()
        .map(|r| {
            let amount: f64 = r.amount.parse().unwrap_or(0.0);
            ShippingRateOption {
                rate_id: r.object_id,
                carrier: r.provider,
                service: r.servicelevel.name,
                price_cents: (amount * 100.0).round() as i32,
                estimated_days: r.estimated_days,
            }
        })
        .collect();

    Ok(Json(rates))
}

async fn buy_label(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<PurchaseLabelRequest>,
) -> AppResult<Json<PurchaseLabelResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    // Verify order exists and is in correct state
    let order = Order::find_by_id(&conn, &id)
        .await?
        .ok_or_else(|| AppError::NotFound("Order not found".to_string()))?;

    let status = OrderStatus::from_str(&order.status);
    match status {
        Some(OrderStatus::Paid) | Some(OrderStatus::Processing) => {
            // OK to buy label
        }
        Some(OrderStatus::Shipped) | Some(OrderStatus::Delivered) => {
            return Err(AppError::BadRequest("Order already shipped".to_string()));
        }
        _ => {
            return Err(AppError::BadRequest("Order not ready for shipping".to_string()));
        }
    }

    if order.label_url.is_some() {
        return Err(AppError::BadRequest("Label already purchased for this order".to_string()));
    }

    // Purchase the label from Shippo
    let transaction = state.shippo.purchase_label(&payload.rate_id).await?;

    let tracking_number = transaction.tracking_number
        .ok_or_else(|| AppError::ExternalService("No tracking number in response".to_string()))?;
    let label_url = transaction.label_url
        .ok_or_else(|| AppError::ExternalService("No label URL in response".to_string()))?;

    // Update order with label info
    Order::set_label(&conn, &id, &tracking_number, &label_url, None).await?;

    // Register tracking with Shippo for webhook updates
    let _ = state.shippo.register_tracking(&tracking_number, "usps").await;

    tracing::info!("Purchased label for order {}: tracking={}", id, tracking_number);

    Ok(Json(PurchaseLabelResponse {
        tracking_number,
        label_url,
        carrier: None,
    }))
}
