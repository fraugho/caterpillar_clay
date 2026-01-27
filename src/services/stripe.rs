use stripe::{
    CheckoutSession, CheckoutSessionMode, Client, CreateCheckoutSession,
    CreateCheckoutSessionLineItems, CreateCheckoutSessionLineItemsPriceData,
    CreateCheckoutSessionLineItemsPriceDataProductData, CreateCheckoutSessionShippingAddressCollection,
    CreateCheckoutSessionShippingAddressCollectionAllowedCountries, CreatePrice,
    CreateProduct, CreateRefund, Currency, IdOrCreate, Price, Product as StripeProduct,
    Refund, UpdatePrice, UpdateProduct,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct StripeService {
    client: Client,
    webhook_secret: String,
}

impl StripeService {
    pub fn new(secret_key: &str, webhook_secret: &str) -> Self {
        Self {
            client: Client::new(secret_key),
            webhook_secret: webhook_secret.to_string(),
        }
    }

    /// Create a product in Stripe, returns (product_id, price_id)
    pub async fn create_product(
        &self,
        name: &str,
        description: Option<&str>,
        price_cents: i64,
        images: Vec<String>,
    ) -> AppResult<(String, String)> {
        // Create product
        let mut create_product = CreateProduct::new(name);
        if let Some(desc) = description {
            create_product.description = Some(desc);
        }
        if !images.is_empty() {
            create_product.images = Some(images);
        }

        let product = StripeProduct::create(&self.client, create_product)
            .await
            .map_err(|e| AppError::ExternalService(format!("Stripe product creation error: {}", e)))?;

        // Create price for the product
        let mut create_price = CreatePrice::new(Currency::USD);
        create_price.product = Some(IdOrCreate::Id(&product.id));
        create_price.unit_amount = Some(price_cents);

        let price = Price::create(&self.client, create_price)
            .await
            .map_err(|e| AppError::ExternalService(format!("Stripe price creation error: {}", e)))?;

        Ok((product.id.to_string(), price.id.to_string()))
    }

    /// Update a product in Stripe
    pub async fn update_product(
        &self,
        product_id: &str,
        name: Option<&str>,
        description: Option<&str>,
        images: Option<Vec<String>>,
    ) -> AppResult<()> {
        let product_id: stripe::ProductId = product_id.parse().map_err(|_| {
            AppError::ExternalService("Invalid Stripe product ID".to_string())
        })?;

        let mut update = UpdateProduct::default();
        if let Some(n) = name {
            update.name = Some(n);
        }
        if let Some(d) = description {
            update.description = Some(d.to_string());
        }
        if let Some(imgs) = images {
            update.images = Some(imgs);
        }

        StripeProduct::update(&self.client, &product_id, update)
            .await
            .map_err(|e| AppError::ExternalService(format!("Stripe product update error: {}", e)))?;

        Ok(())
    }

    /// Update product price (creates new price, archives old one)
    pub async fn update_price(
        &self,
        product_id: &str,
        new_price_cents: i64,
        old_price_id: Option<&str>,
    ) -> AppResult<String> {
        let product_id_parsed: stripe::ProductId = product_id.parse().map_err(|_| {
            AppError::ExternalService("Invalid Stripe product ID".to_string())
        })?;

        // Create new price
        let mut create_price = CreatePrice::new(Currency::USD);
        create_price.product = Some(IdOrCreate::Id(&product_id_parsed));
        create_price.unit_amount = Some(new_price_cents);

        let price = Price::create(&self.client, create_price)
            .await
            .map_err(|e| AppError::ExternalService(format!("Stripe price creation error: {}", e)))?;

        // Archive the old price if provided
        if let Some(old_id) = old_price_id {
            tracing::info!("Archiving old Stripe price: {}", old_id);
            match old_id.parse::<stripe::PriceId>() {
                Ok(old_price_id_parsed) => {
                    let mut update = UpdatePrice::default();
                    update.active = Some(false);
                    match Price::update(&self.client, &old_price_id_parsed, update).await {
                        Ok(_) => tracing::info!("Successfully archived old price {}", old_id),
                        Err(e) => tracing::error!("Failed to archive old price {}: {}", old_id, e),
                    }
                }
                Err(e) => tracing::error!("Failed to parse old price ID {}: {}", old_id, e),
            }
        }

        Ok(price.id.to_string())
    }

    /// Archive a product in Stripe (set active = false)
    pub async fn archive_product(&self, product_id: &str) -> AppResult<()> {
        let product_id: stripe::ProductId = product_id.parse().map_err(|_| {
            AppError::ExternalService("Invalid Stripe product ID".to_string())
        })?;

        let mut update = UpdateProduct::default();
        update.active = Some(false);

        StripeProduct::update(&self.client, &product_id, update)
            .await
            .map_err(|e| AppError::ExternalService(format!("Stripe archive error: {}", e)))?;

        Ok(())
    }

    /// Create a checkout session for an order
    pub async fn create_checkout_session(
        &self,
        items: Vec<CheckoutItem>,
        success_url: &str,
        cancel_url: &str,
        customer_email: Option<&str>,
        order_id: &str,
    ) -> AppResult<CheckoutSessionResult> {
        let line_items: Vec<CreateCheckoutSessionLineItems> = items
            .into_iter()
            .map(|item| {
                let mut line_item = CreateCheckoutSessionLineItems::default();
                line_item.price_data = Some(CreateCheckoutSessionLineItemsPriceData {
                    currency: Currency::USD,
                    product_data: Some(CreateCheckoutSessionLineItemsPriceDataProductData {
                        name: item.name,
                        description: item.description,
                        images: item.images,
                        ..Default::default()
                    }),
                    unit_amount: Some(item.price_cents),
                    ..Default::default()
                });
                line_item.quantity = Some(item.quantity as u64);
                line_item
            })
            .collect();

        let mut params = CreateCheckoutSession::new();
        params.line_items = Some(line_items);
        params.mode = Some(CheckoutSessionMode::Payment);
        params.success_url = Some(success_url);
        params.cancel_url = Some(cancel_url);

        if let Some(email) = customer_email {
            params.customer_email = Some(email);
        }

        // Add shipping address collection for physical goods
        params.shipping_address_collection = Some(CreateCheckoutSessionShippingAddressCollection {
            allowed_countries: vec![
                CreateCheckoutSessionShippingAddressCollectionAllowedCountries::Us,
            ],
        });

        // Store order ID in metadata
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("order_id".to_string(), order_id.to_string());
        params.metadata = Some(metadata);

        let session = CheckoutSession::create(&self.client, params)
            .await
            .map_err(|e| AppError::ExternalService(format!("Stripe checkout error: {}", e)))?;

        Ok(CheckoutSessionResult {
            id: session.id.to_string(),
            url: session.url.ok_or_else(|| {
                AppError::ExternalService("No checkout URL returned".to_string())
            })?,
        })
    }

    /// Create a refund for a payment intent
    /// Returns the refund ID if successful
    pub async fn create_refund(
        &self,
        payment_intent_id: &str,
        amount_cents: Option<i64>,
        reason: Option<&str>,
    ) -> AppResult<RefundResult> {
        let pi_id: stripe::PaymentIntentId = payment_intent_id.parse().map_err(|_| {
            AppError::ExternalService("Invalid payment intent ID".to_string())
        })?;

        let mut params = CreateRefund::default();
        params.payment_intent = Some(pi_id);

        // If amount is specified, do partial refund; otherwise full refund
        if let Some(amount) = amount_cents {
            params.amount = Some(amount);
        }

        // Map reason string to Stripe RefundReasonFilter enum
        if let Some(r) = reason {
            params.reason = match r {
                "duplicate" => Some(stripe::RefundReasonFilter::Duplicate),
                "fraudulent" => Some(stripe::RefundReasonFilter::Fraudulent),
                _ => Some(stripe::RefundReasonFilter::RequestedByCustomer),
            };
        }

        let refund = Refund::create(&self.client, params)
            .await
            .map_err(|e| AppError::ExternalService(format!("Stripe refund error: {}", e)))?;

        Ok(RefundResult {
            id: refund.id.to_string(),
            status: refund.status.map(|s| format!("{:?}", s).to_lowercase()).unwrap_or_default(),
            amount: refund.amount,
        })
    }

    /// Verify webhook signature and parse event
    pub fn verify_webhook(&self, payload: &str, signature: &str) -> AppResult<StripeWebhookEvent> {
        // Parse the Stripe-Signature header
        let mut timestamp: Option<i64> = None;
        let mut signatures: Vec<String> = Vec::new();

        for part in signature.split(',') {
            let kv: Vec<&str> = part.split('=').collect();
            if kv.len() == 2 {
                match kv[0] {
                    "t" => timestamp = kv[1].parse().ok(),
                    "v1" => signatures.push(kv[1].to_string()),
                    _ => {}
                }
            }
        }

        let timestamp = timestamp.ok_or_else(|| {
            AppError::ExternalService("Missing timestamp in webhook signature".to_string())
        })?;

        if signatures.is_empty() {
            return Err(AppError::ExternalService("Missing signature in webhook header".to_string()));
        }

        // Create signed payload
        let signed_payload = format!("{}.{}", timestamp, payload);

        // Verify signature using HMAC-SHA256
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(self.webhook_secret.as_bytes())
            .map_err(|e| AppError::ExternalService(format!("HMAC error: {}", e)))?;
        mac.update(signed_payload.as_bytes());

        let expected = hex::encode(mac.finalize().into_bytes());

        let valid = signatures.iter().any(|sig| sig == &expected);
        if !valid {
            return Err(AppError::ExternalService("Invalid webhook signature".to_string()));
        }

        // Check timestamp tolerance (5 minutes)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        if (now - timestamp).abs() > 300 {
            return Err(AppError::ExternalService("Webhook timestamp too old".to_string()));
        }

        // Parse the event
        serde_json::from_str(payload)
            .map_err(|e| AppError::ExternalService(format!("Failed to parse webhook event: {}", e)))
    }
}

/// Stripe webhook event structure
#[derive(Debug, serde::Deserialize)]
pub struct StripeWebhookEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: StripeWebhookData,
}

#[derive(Debug, serde::Deserialize)]
pub struct StripeWebhookData {
    pub object: serde_json::Value,
}

pub struct CheckoutItem {
    pub name: String,
    pub description: Option<String>,
    pub images: Option<Vec<String>>,
    pub price_cents: i64,
    pub quantity: i32,
}

pub struct CheckoutSessionResult {
    pub id: String,
    pub url: String,
}

pub struct RefundResult {
    pub id: String,
    pub status: String,
    pub amount: i64,
}
