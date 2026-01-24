use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

use crate::error::AppResult;
use crate::models::{Order, Product};
use crate::routes::AppState;

#[derive(Serialize)]
pub struct DashboardStats {
    pub total_orders: i64,
    pub total_revenue_cents: i64,
    pub total_revenue: f64,
    pub total_products: i64,
    pub low_stock_products: Vec<LowStockProduct>,
    pub recent_orders: Vec<RecentOrder>,
}

#[derive(Serialize)]
pub struct LowStockProduct {
    pub id: String,
    pub name: String,
    pub stock_quantity: i32,
}

#[derive(Serialize)]
pub struct RecentOrder {
    pub id: String,
    pub status: String,
    pub total: f64,
    pub created_at: String,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/dashboard", get(get_dashboard))
}

async fn get_dashboard(State(state): State<AppState>) -> AppResult<Json<DashboardStats>> {
    let total_orders = Order::count_all(&state.pool).await?;
    let total_revenue_cents = Order::total_revenue(&state.pool).await?;

    let products = Product::list_all(&state.pool).await?;
    let total_products = products.len() as i64;

    let low_stock_products: Vec<LowStockProduct> = products
        .iter()
        .filter(|p| p.is_active && p.stock_quantity < 5)
        .map(|p| LowStockProduct {
            id: p.id.to_string(),
            name: p.name.clone(),
            stock_quantity: p.stock_quantity,
        })
        .collect();

    let orders = Order::list_all(&state.pool).await?;
    let recent_orders: Vec<RecentOrder> = orders
        .into_iter()
        .take(10)
        .map(|o| RecentOrder {
            id: o.id.to_string()[..8].to_string(),
            status: o.status,
            total: o.total_cents as f64 / 100.0,
            created_at: o.created_at,
        })
        .collect();

    Ok(Json(DashboardStats {
        total_orders,
        total_revenue_cents,
        total_revenue: total_revenue_cents as f64 / 100.0,
        total_products,
        low_stock_products,
        recent_orders,
    }))
}
