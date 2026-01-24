use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    Pending,
    Paid,
    Processing,
    Shipped,
    Delivered,
    Cancelled,
}

impl OrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderStatus::Pending => "pending",
            OrderStatus::Paid => "paid",
            OrderStatus::Processing => "processing",
            OrderStatus::Shipped => "shipped",
            OrderStatus::Delivered => "delivered",
            OrderStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(OrderStatus::Pending),
            "paid" => Some(OrderStatus::Paid),
            "processing" => Some(OrderStatus::Processing),
            "shipped" => Some(OrderStatus::Shipped),
            "delivered" => Some(OrderStatus::Delivered),
            "cancelled" => Some(OrderStatus::Cancelled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShippingAddress {
    pub name: String,
    pub street: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub country: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Order {
    pub id: String,
    pub user_id: Option<String>,
    pub status: String,
    pub total_cents: i32,
    pub shipping_address: String,
    pub tracking_number: Option<String>,
    pub easypost_tracker_id: Option<String>,
    pub polar_checkout_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OrderItem {
    pub id: String,
    pub order_id: String,
    pub product_id: String,
    pub quantity: i32,
    pub price_cents: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderItem {
    pub product_id: String,
    pub quantity: i32,
    pub price_cents: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrder {
    pub user_id: Option<String>,
    pub total_cents: i32,
    pub shipping_address: ShippingAddress,
    pub polar_checkout_id: Option<String>,
    pub items: Vec<CreateOrderItem>,
}

impl Order {
    pub fn uuid(&self) -> Option<Uuid> {
        Uuid::parse_str(&self.id).ok()
    }

    pub fn get_shipping_address(&self) -> Option<ShippingAddress> {
        serde_json::from_str(&self.shipping_address).ok()
    }

    pub fn get_status(&self) -> Option<OrderStatus> {
        OrderStatus::from_str(&self.status)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> AppResult<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM orders WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .map_err(AppError::from)
    }

    pub async fn find_by_polar_checkout(pool: &SqlitePool, checkout_id: &str) -> AppResult<Option<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM orders WHERE polar_checkout_id = ?")
            .bind(checkout_id)
            .fetch_optional(pool)
            .await
            .map_err(AppError::from)
    }

    pub async fn list_by_user(pool: &SqlitePool, user_id: &str) -> AppResult<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM orders WHERE user_id = ? ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
    }

    pub async fn list_all(pool: &SqlitePool) -> AppResult<Vec<Self>> {
        sqlx::query_as::<_, Self>("SELECT * FROM orders ORDER BY created_at DESC")
            .fetch_all(pool)
            .await
            .map_err(AppError::from)
    }

    pub async fn create(pool: &SqlitePool, data: CreateOrder) -> AppResult<Self> {
        let id = Uuid::new_v4().to_string();
        let shipping_json = serde_json::to_string(&data.shipping_address)
            .map_err(|e| AppError::Internal(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO orders (id, user_id, total_cents, shipping_address, polar_checkout_id)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&data.user_id)
        .bind(data.total_cents)
        .bind(&shipping_json)
        .bind(&data.polar_checkout_id)
        .execute(pool)
        .await
        .map_err(AppError::from)?;

        for item in data.items {
            let item_id = Uuid::new_v4().to_string();
            sqlx::query(
                r#"
                INSERT INTO order_items (id, order_id, product_id, quantity, price_cents)
                VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(&item_id)
            .bind(&id)
            .bind(&item.product_id)
            .bind(item.quantity)
            .bind(item.price_cents)
            .execute(pool)
            .await
            .map_err(AppError::from)?;
        }

        Self::find_by_id(pool, &id)
            .await?
            .ok_or_else(|| AppError::Internal("Failed to create order".to_string()))
    }

    pub async fn update_status(pool: &SqlitePool, id: &str, status: OrderStatus) -> AppResult<Self> {
        sqlx::query(
            r#"
            UPDATE orders SET status = ?, updated_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(status.as_str())
        .bind(id)
        .execute(pool)
        .await
        .map_err(AppError::from)?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Order not found".to_string()))
    }

    pub async fn set_tracking(
        pool: &SqlitePool,
        id: &str,
        tracking_number: &str,
        easypost_tracker_id: Option<&str>,
    ) -> AppResult<Self> {
        sqlx::query(
            r#"
            UPDATE orders SET
                tracking_number = ?,
                easypost_tracker_id = ?,
                status = 'shipped',
                updated_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(tracking_number)
        .bind(easypost_tracker_id)
        .bind(id)
        .execute(pool)
        .await
        .map_err(AppError::from)?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Order not found".to_string()))
    }

    pub async fn get_items(pool: &SqlitePool, order_id: &str) -> AppResult<Vec<OrderItem>> {
        sqlx::query_as::<_, OrderItem>("SELECT * FROM order_items WHERE order_id = ?")
            .bind(order_id)
            .fetch_all(pool)
            .await
            .map_err(AppError::from)
    }

    pub async fn count_all(pool: &SqlitePool) -> AppResult<i64> {
        let row: (i32,) = sqlx::query_as("SELECT COUNT(*) FROM orders")
            .fetch_one(pool)
            .await
            .map_err(AppError::from)?;
        Ok(row.0 as i64)
    }

    pub async fn total_revenue(pool: &SqlitePool) -> AppResult<i64> {
        let row: (Option<i64>,) = sqlx::query_as(
            "SELECT SUM(total_cents) FROM orders WHERE status NOT IN ('pending', 'cancelled')",
        )
        .fetch_one(pool)
        .await
        .map_err(AppError::from)?;
        Ok(row.0.unwrap_or(0))
    }
}
