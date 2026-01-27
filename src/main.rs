mod config;
mod db;
mod error;
mod middleware;
mod models;
mod routes;
mod services;
mod storage;

use std::sync::Arc;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;
use crate::routes::{create_router, AppState};
use crate::services::{ClerkService, EmailService, JwksVerifier, RateLimiter, ResendService, ShippoService, StripeService};
use crate::storage::{LocalStorage, R2Storage, StorageBackend};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Debug output to stderr (Cloud Run captures this)
    eprintln!("=== Caterpillar Clay starting ===");

    // Install rustls crypto provider before any TLS connections
    eprintln!("Installing rustls crypto provider...");
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
    eprintln!("Rustls crypto provider installed");

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "caterpillar_clay=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    eprintln!("Loading environment variables...");
    dotenvy::dotenv().ok();

    // Load configuration
    eprintln!("Loading configuration...");
    let config = Config::from_env().expect("Failed to load configuration");
    eprintln!("Configuration loaded, port={}", config.port);

    if config.testing_mode {
        tracing::warn!("TESTING MODE - Using test API keys and database");
    } else {
        tracing::info!("PRODUCTION MODE - Using production API keys and database");
    }

    // Create database connection
    eprintln!("Connecting to database: {}...", &config.database_url[..50.min(config.database_url.len())]);
    let db = db::create_database(&config.database_url, config.turso_auth_token.as_deref())
        .await
        .expect("Failed to create database");
    eprintln!("Database connected");

    tracing::info!("Connected to database");

    // Initialize services
    let clerk = ClerkService::new(&config.clerk_secret_key);
    let jwks = JwksVerifier::new(&config.clerk_jwks_url);

    // Initialize JWKS cache (fetch keys on startup)
    if let Err(e) = jwks.initialize().await {
        tracing::warn!("Failed to initialize JWKS cache: {} - will retry on first request", e);
    } else {
        tracing::info!("JWKS cache initialized");
    }

    let stripe = StripeService::new(&config.stripe_secret_key, &config.stripe_webhook_secret);
    let shippo = ShippoService::new(&config.shippo_api_key);

    // Initialize Upstash rate limiter if configured
    let rate_limiter = match &config.upstash_redis_url {
        Some(url) => {
            match RateLimiter::new(url, config.rate_limit_general) {
                Ok(limiter) => {
                    tracing::info!("Upstash Redis rate limiter configured");
                    Some(limiter)
                }
                Err(e) => {
                    tracing::error!("Failed to initialize rate limiter: {} - rate limiting disabled", e);
                    None
                }
            }
        }
        None => {
            tracing::warn!("Upstash Redis not configured - rate limiting disabled");
            None
        }
    };

    let email = match EmailService::new(
        &config.smtp_host,
        &config.smtp_user,
        &config.smtp_pass,
        &config.from_email,
    ) {
        Ok(service) => {
            tracing::info!("Email service initialized");
            Some(service)
        }
        Err(e) => {
            tracing::warn!("Email service not available: {}", e);
            None
        }
    };

    // Initialize Resend service for newsletters
    let resend = config.resend_api_key.as_ref().map(|api_key| {
        tracing::info!("Resend newsletter service initialized");
        ResendService::new(api_key, &config.from_email, &config.base_url)
    });

    // Initialize storage
    let storage: Arc<dyn StorageBackend> = if config.storage_type == "r2" {
        match (&config.r2_bucket, &config.r2_account_id, &config.r2_access_key, &config.r2_secret_key, &config.r2_public_url) {
            (Some(bucket), Some(account_id), Some(access_key), Some(secret_key), Some(public_url)) => {
                tracing::info!("Using R2 storage");
                Arc::new(R2Storage::new(bucket, account_id, access_key, secret_key, public_url)
                    .expect("Failed to initialize R2 storage"))
            }
            _ => {
                tracing::warn!("R2 storage configured but missing credentials, falling back to local");
                let local = LocalStorage::new(&config.upload_dir, &config.base_url);
                local.ensure_dir().await.expect("Failed to create upload directory");
                Arc::new(local)
            }
        }
    } else {
        tracing::info!("Using local storage");
        let local = LocalStorage::new(&config.upload_dir, &config.base_url);
        local.ensure_dir().await.expect("Failed to create upload directory");
        Arc::new(local)
    };

    // Create app state
    let state = AppState {
        db: Arc::new(db),
        config: config.clone(),
        clerk,
        jwks,
        stripe,
        shippo,
        email,
        resend,
        storage,
        rate_limiter,
    };

    // Create router
    let app = create_router(state);

    // Start server
    let addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(&addr).await?;

    tracing::info!("Server listening on {}", addr);
    tracing::info!("Admin panel: {}/gallium/", config.base_url);

    axum::serve(listener, app).await?;

    Ok(())
}
