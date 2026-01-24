use libsql::Connection;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Order {
    fn from_row(row: &libsql::Row) -> Result<Self, libsql::Error> {
        Ok(Self {
            id: row.get(0)?,
            user_id: row.get(1)?,
            status: row.get(2)?,
            total_cents: row.get(3)?,
            shipping_address: row.get(4)?,
            tracking_number: row.get(5)?,
            easypost_tracker_id: row.get(6)?,
            polar_checkout_id: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub id: String,
    pub order_id: String,
    pub product_id: String,
    pub quantity: i32,
    pub price_cents: i32,
}

impl OrderItem {
    fn from_row(row: &libsql::Row) -> Result<Self, libsql::Error> {
        Ok(Self {
            id: row.get(0)?,
            order_id: row.get(1)?,
            product_id: row.get(2)?,
            quantity: row.get(3)?,
            price_cents: row.get(4)?,
        })
    }
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

    pub async fn find_by_id(conn: &Connection, id: &str) -> AppResult<Option<Self>> {
        let mut rows = conn
            .query("SELECT * FROM orders WHERE id = ?", [id])
            .await
            .map_err(AppError::from)?;

        match rows.next().await.map_err(AppError::from)? {
            Some(row) => Ok(Some(Self::from_row(&row).map_err(AppError::from)?)),
            None => Ok(None),
        }
    }

    pub async fn find_by_polar_checkout(conn: &Connection, checkout_id: &str) -> AppResult<Option<Self>> {
        let mut rows = conn
            .query("SELECT * FROM orders WHERE polar_checkout_id = ?", [checkout_id])
            .await
            .map_err(AppError::from)?;

        match rows.next().await.map_err(AppError::from)? {
            Some(row) => Ok(Some(Self::from_row(&row).map_err(AppError::from)?)),
            None => Ok(None),
        }
    }

    pub async fn list_by_user(conn: &Connection, user_id: &str) -> AppResult<Vec<Self>> {
        let mut rows = conn
            .query(
                "SELECT * FROM orders WHERE user_id = ? ORDER BY created_at DESC",
                [user_id],
            )
            .await
            .map_err(AppError::from)?;

        let mut orders = Vec::new();
        while let Some(row) = rows.next().await.map_err(AppError::from)? {
            orders.push(Self::from_row(&row).map_err(AppError::from)?);
        }
        Ok(orders)
    }

    pub async fn list_all(conn: &Connection) -> AppResult<Vec<Self>> {
        let mut rows = conn
            .query("SELECT * FROM orders ORDER BY created_at DESC", ())
            .await
            .map_err(AppError::from)?;

        let mut orders = Vec::new();
        while let Some(row) = rows.next().await.map_err(AppError::from)? {
            orders.push(Self::from_row(&row).map_err(AppError::from)?);
        }
        Ok(orders)
    }

    pub async fn create(conn: &Connection, data: CreateOrder) -> AppResult<Self> {
        let id = Uuid::new_v4().to_string();
        let shipping_json = serde_json::to_string(&data.shipping_address)
            .map_err(|e| AppError::Internal(e.to_string()))?;

        conn.execute(
            "INSERT INTO orders (id, user_id, total_cents, shipping_address, polar_checkout_id) VALUES (?, ?, ?, ?, ?)",
            libsql::params![id.clone(), data.user_id.clone(), data.total_cents, shipping_json, data.polar_checkout_id.clone()],
        )
        .await
        .map_err(AppError::from)?;

        for item in data.items {
            let item_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO order_items (id, order_id, product_id, quantity, price_cents) VALUES (?, ?, ?, ?, ?)",
                libsql::params![item_id, id.clone(), item.product_id, item.quantity, item.price_cents],
            )
            .await
            .map_err(AppError::from)?;
        }

        Self::find_by_id(conn, &id)
            .await?
            .ok_or_else(|| AppError::Internal("Failed to create order".to_string()))
    }

    pub async fn update_status(conn: &Connection, id: &str, status: OrderStatus) -> AppResult<Self> {
        conn.execute(
            "UPDATE orders SET status = ?, updated_at = datetime('now') WHERE id = ?",
            libsql::params![status.as_str().to_string(), id.to_string()],
        )
        .await
        .map_err(AppError::from)?;

        Self::find_by_id(conn, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Order not found".to_string()))
    }

    pub async fn set_tracking(
        conn: &Connection,
        id: &str,
        tracking_number: &str,
        easypost_tracker_id: Option<&str>,
    ) -> AppResult<Self> {
        conn.execute(
            r#"
            UPDATE orders SET
                tracking_number = ?,
                easypost_tracker_id = ?,
                status = 'shipped',
                updated_at = datetime('now')
            WHERE id = ?
            "#,
            libsql::params![tracking_number.to_string(), easypost_tracker_id.map(|s| s.to_string()), id.to_string()],
        )
        .await
        .map_err(AppError::from)?;

        Self::find_by_id(conn, id)
            .await?
            .ok_or_else(|| AppError::NotFound("Order not found".to_string()))
    }

    pub async fn get_items(conn: &Connection, order_id: &str) -> AppResult<Vec<OrderItem>> {
        let mut rows = conn
            .query("SELECT * FROM order_items WHERE order_id = ?", [order_id])
            .await
            .map_err(AppError::from)?;

        let mut items = Vec::new();
        while let Some(row) = rows.next().await.map_err(AppError::from)? {
            items.push(OrderItem::from_row(&row).map_err(AppError::from)?);
        }
        Ok(items)
    }

    pub async fn count_all(conn: &Connection) -> AppResult<i64> {
        let mut rows = conn
            .query("SELECT COUNT(*) FROM orders", ())
            .await
            .map_err(AppError::from)?;

        match rows.next().await.map_err(AppError::from)? {
            Some(row) => {
                let count: i32 = row.get(0).map_err(AppError::from)?;
                Ok(count as i64)
            }
            None => Ok(0),
        }
    }

    pub async fn total_revenue(conn: &Connection) -> AppResult<i64> {
        let mut rows = conn
            .query(
                "SELECT SUM(total_cents) FROM orders WHERE status NOT IN ('pending', 'cancelled')",
                (),
            )
            .await
            .map_err(AppError::from)?;

        match rows.next().await.map_err(AppError::from)? {
            Some(row) => {
                let total: Option<i64> = row.get(0).map_err(AppError::from)?;
                Ok(total.unwrap_or(0))
            }
            None => Ok(0),
        }
    }
}
