use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use serde_json::json;

use crate::models::{Order, OrderStatus, Product, User};
use crate::routes::AppState;
use crate::services::easypost::EasyPostWebhookEvent;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/webhooks/stripe", post(stripe_webhook))
        .route("/webhooks/easypost", post(easypost_webhook))
}

async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, Json<serde_json::Value>) {
    // Get Stripe signature header
    let signature = match headers.get("stripe-signature").and_then(|h| h.to_str().ok()) {
        Some(sig) => sig,
        None => {
            tracing::error!("Missing Stripe signature header");
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Missing signature"})),
            );
        }
    };

    // Convert body to string for verification
    let payload = match std::str::from_utf8(&body) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Invalid UTF-8 in webhook body: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Invalid payload"})),
            );
        }
    };

    // Verify webhook signature
    let event = match state.stripe.verify_webhook(payload, signature) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("Stripe webhook verification failed: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Invalid signature"})),
            );
        }
    };

    tracing::info!("Received Stripe webhook: {}", event.event_type);

    let conn = match state.db.connect() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Database connection error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            );
        }
    };

    match event.event_type.as_str() {
        "checkout.session.completed" => {
            // Get order_id from metadata
            let order_id = event.data.object
                .get("metadata")
                .and_then(|m| m.get("order_id"))
                .and_then(|v| v.as_str());

            if let Some(order_id) = order_id {
                match Order::find_by_id(&conn, order_id).await {
                    Ok(Some(order)) => {
                        // Update order status to paid
                        if let Err(e) = Order::update_status(&conn, &order.id, OrderStatus::Paid).await {
                            tracing::error!("Failed to update order status: {}", e);
                        }

                        // Decrement stock
                        if let Ok(items) = Order::get_items(&conn, &order.id).await {
                            for item in items {
                                let _ = Product::decrement_stock(&conn, &item.product_id, item.quantity).await;
                            }
                        }

                        // Send confirmation email
                        if let Some(ref email_service) = state.email {
                            if let Some(ref user_id) = order.user_id {
                                if let Ok(Some(user)) = User::find_by_id(&conn, user_id).await {
                                    let name = user.name.as_deref().unwrap_or("Customer");
                                    let _ = email_service
                                        .send_order_confirmation(&user.email, &order, name)
                                        .await;
                                }
                            }
                        }

                        tracing::info!("Order {} marked as paid via Stripe", order.id);
                    }
                    Ok(None) => {
                        tracing::warn!("Order not found: {}", order_id);
                    }
                    Err(e) => {
                        tracing::error!("Database error: {}", e);
                    }
                }
            } else {
                tracing::warn!("No order_id in checkout session metadata");
            }
        }
        _ => {
            tracing::debug!("Unhandled Stripe event type: {}", event.event_type);
        }
    }

    (StatusCode::OK, Json(json!({"received": true})))
}

async fn easypost_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, Json<serde_json::Value>) {
    // Verify webhook signature (in production)
    let _signature = headers
        .get("x-easypost-signature")
        .and_then(|h| h.to_str().ok());

    let event: EasyPostWebhookEvent = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("Failed to parse EasyPost webhook: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Invalid payload"})),
            );
        }
    };

    tracing::info!("Received EasyPost webhook: {}", event.description);

    let conn = match state.db.connect() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Database connection error: {}", e);
            return (StatusCode::OK, Json(json!({"received": true})));
        }
    };

    let tracker = &event.result;

    // Find order by tracker ID
    let orders = match Order::list_all(&conn).await {
        Ok(o) => o,
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return (StatusCode::OK, Json(json!({"received": true})));
        }
    };

    let order = orders
        .into_iter()
        .find(|o| o.easypost_tracker_id.as_deref() == Some(&tracker.id));

    if let Some(order) = order {
        let new_status = match tracker.status.as_str() {
            "delivered" => Some(OrderStatus::Delivered),
            "in_transit" | "out_for_delivery" => Some(OrderStatus::Shipped),
            _ => None,
        };

        if let Some(status) = new_status {
            if let Err(e) = Order::update_status(&conn, &order.id, status).await {
                tracing::error!("Failed to update order status: {}", e);
            } else {
                tracing::info!("Order {} status updated to {:?}", order.id, status);

                // Send delivery email
                if status == OrderStatus::Delivered {
                    if let Some(ref email_service) = state.email {
                        if let Some(ref user_id) = order.user_id {
                            if let Ok(Some(user)) = User::find_by_id(&conn, user_id).await {
                                let name = user.name.as_deref().unwrap_or("Customer");
                                let _ = email_service
                                    .send_order_delivered(&user.email, &order, name)
                                    .await;
                            }
                        }
                    }
                }
            }
        }
    }

    (StatusCode::OK, Json(json!({"received": true})))
}
