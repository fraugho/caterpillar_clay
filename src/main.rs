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
use crate::services::{ClerkService, EasyPostService, EmailService, PolarService};
use crate::storage::{LocalStorage, R2Storage, StorageBackend};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "caterpillar_clay=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Load configuration
    let config = Config::from_env().expect("Failed to load configuration");

    // Create database connection
    let db = db::create_database(&config.database_url)
        .await
        .expect("Failed to create database");

    tracing::info!("Connected to database");

    // Initialize services
    let clerk = ClerkService::new(&config.clerk_secret_key);
    let polar = PolarService::new(&config.polar_access_token);
    let easypost = EasyPostService::new(&config.easypost_api_key);

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
        polar,
        easypost,
        email,
        storage,
    };

    // Create router
    let app = create_router(state);

    // Start server
    let addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(&addr).await?;

    tracing::info!("Server listening on {}", addr);
    tracing::info!("Admin panel: {}/admin/", config.base_url);

    axum::serve(listener, app).await?;

    Ok(())
}
