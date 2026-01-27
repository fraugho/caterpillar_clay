use axum::{
    extract::State,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::models::{Product, Setting, ShippingAddress};
use crate::routes::AppState;
use crate::services::shippo::{ShippoAddress, ShippoParcel};

#[derive(Deserialize)]
pub struct ShippingRateItem {
    pub product_id: String,
    pub quantity: i32,
}

#[derive(Deserialize)]
pub struct GetShippingRatesRequest {
    pub items: Vec<ShippingRateItem>,
    pub destination: ShippingAddress,
}

#[derive(Serialize)]
pub struct ShippingRateOption {
    pub rate_id: String,
    pub carrier: String,
    pub service: String,
    pub price_cents: i32,
    pub estimated_days: Option<i32>,
    pub duration_terms: Option<String>,
}

#[derive(Serialize)]
pub struct GetShippingRatesResponse {
    pub rates: Vec<ShippingRateOption>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/shipping/rates", post(get_shipping_rates))
}

async fn get_shipping_rates(
    State(state): State<AppState>,
    Json(payload): Json<GetShippingRatesRequest>,
) -> AppResult<Json<GetShippingRatesResponse>> {
    let conn = state.db.connect().map_err(AppError::from)?;

    // Get shop address
    let shop_address = Setting::get_shop_address(&conn)
        .await?
        .ok_or_else(|| AppError::BadRequest("Shop address not configured. Please set up shipping origin in admin panel.".to_string()))?;

    // Get unit system preference
    let unit_system = Setting::get_unit_system(&conn).await?;
    let (distance_unit, mass_unit) = if unit_system == "metric" {
        ("cm", "g")
    } else {
        ("in", "oz")
    };

    // Calculate total parcel dimensions from cart items
    let mut total_weight = 0.0f64;
    let mut max_length = 0.0f64;
    let mut max_width = 0.0f64;
    let mut total_height = 0.0f64;

    for item in &payload.items {
        let product = Product::find_by_id(&conn, &item.product_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Product {} not found", item.product_id)))?;

        // Use product dimensions or defaults (500g, 15x15x10cm)
        let weight = product.weight_grams.unwrap_or(500) as f64;
        let length = product.length_cm.unwrap_or(15.0);
        let width = product.width_cm.unwrap_or(15.0);
        let height = product.height_cm.unwrap_or(10.0);

        // Accumulate for parcel calculation
        total_weight += weight * item.quantity as f64;
        max_length = max_length.max(length);
        max_width = max_width.max(width);
        total_height += height * item.quantity as f64;
    }

    // Convert units if US system
    let (final_weight, final_length, final_width, final_height) = if unit_system == "us" {
        (
            total_weight * 0.035274,  // grams to oz
            max_length * 0.393701,     // cm to inches
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
        name: payload.destination.name.clone(),
        street1: payload.destination.street.clone(),
        street2: None,
        city: payload.destination.city.clone(),
        state: payload.destination.state.clone(),
        zip: payload.destination.zip.clone(),
        country: payload.destination.country.clone(),
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
                duration_terms: r.duration_terms,
            }
        })
        .collect();

    Ok(Json(GetShippingRatesResponse { rates }))
}
