use axum::{
    extract::{Query, State},
    response::{Html, Redirect},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::models::NewsletterSubscriber;
use crate::routes::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/newsletter/subscribe", post(subscribe))
        .route("/newsletter/unsubscribe", get(unsubscribe))
}

#[derive(Deserialize)]
pub struct SubscribeRequest {
    pub email: String,
}

#[derive(Serialize)]
pub struct SubscribeResponse {
    pub success: bool,
    pub message: String,
}

async fn subscribe(
    State(state): State<AppState>,
    Json(payload): Json<SubscribeRequest>,
) -> AppResult<Json<SubscribeResponse>> {
    // Basic email validation
    if !payload.email.contains('@') || payload.email.len() < 5 {
        return Ok(Json(SubscribeResponse {
            success: false,
            message: "Please enter a valid email address".to_string(),
        }));
    }

    let conn = state.db.connect().map_err(AppError::from)?;
    let subscriber = NewsletterSubscriber::subscribe(&conn, &payload.email).await?;

    // Send welcome email if Resend is configured
    if let Some(resend) = &state.resend {
        if let Err(e) = resend.send_welcome_email(&subscriber.email, &subscriber.unsubscribe_token).await {
            tracing::error!("Failed to send welcome email: {}", e);
        }
    }

    Ok(Json(SubscribeResponse {
        success: true,
        message: "Thanks for subscribing! You'll be notified when we add new items.".to_string(),
    }))
}

#[derive(Deserialize)]
pub struct UnsubscribeQuery {
    pub token: String,
}

async fn unsubscribe(
    State(state): State<AppState>,
    Query(query): Query<UnsubscribeQuery>,
) -> AppResult<Html<String>> {
    let conn = state.db.connect().map_err(AppError::from)?;
    let success = NewsletterSubscriber::unsubscribe_by_token(&conn, &query.token).await?;

    let html = if success {
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Unsubscribed - Caterpillar Clay</title>
    <link href="https://fonts.googleapis.com/css2?family=Press+Start+2P&display=swap" rel="stylesheet">
    <style>
        body { font-family: 'Press Start 2P', cursive; background: #F8F8F8; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; }
        .container { background: white; padding: 40px; border: 2px solid #E0E0E0; border-radius: 12px; text-align: center; max-width: 400px; }
        h1 { color: #97BAD9; font-size: 14px; margin-bottom: 20px; }
        p { font-size: 10px; color: #666; line-height: 2; margin-bottom: 20px; }
        a { display: inline-block; background: #97BAD9; color: #18191B; padding: 14px 24px; text-decoration: none; font-size: 10px; border-radius: 8px; font-family: inherit; }
    </style>
</head>
<body>
    <div class="container">
        <h1>Unsubscribed</h1>
        <p>You've been removed from our newsletter. We're sorry to see you go!</p>
        <a href="/">Back to Shop</a>
    </div>
</body>
</html>"#.to_string()
    } else {
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Unsubscribe - Caterpillar Clay</title>
    <link href="https://fonts.googleapis.com/css2?family=Press+Start+2P&display=swap" rel="stylesheet">
    <style>
        body { font-family: 'Press Start 2P', cursive; background: #F8F8F8; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; }
        .container { background: white; padding: 40px; border: 2px solid #E0E0E0; border-radius: 12px; text-align: center; max-width: 400px; }
        h1 { color: #97BAD9; font-size: 14px; margin-bottom: 20px; }
        p { font-size: 10px; color: #666; line-height: 2; margin-bottom: 20px; }
        a { display: inline-block; background: #97BAD9; color: #18191B; padding: 14px 24px; text-decoration: none; font-size: 10px; border-radius: 8px; font-family: inherit; }
    </style>
</head>
<body>
    <div class="container">
        <h1>Already Unsubscribed</h1>
        <p>This email is not subscribed to our newsletter.</p>
        <a href="/">Back to Shop</a>
    </div>
</body>
</html>"#.to_string()
    };

    Ok(Html(html))
}
