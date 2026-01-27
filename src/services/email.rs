use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

use crate::error::{AppError, AppResult};
use crate::models::Order;

#[derive(Clone)]
pub struct EmailService {
    mailer: AsyncSmtpTransport<Tokio1Executor>,
    from_email: String,
}

impl EmailService {
    pub fn new(smtp_host: &str, smtp_user: &str, smtp_pass: &str, from_email: &str) -> AppResult<Self> {
        let creds = Credentials::new(smtp_user.to_string(), smtp_pass.to_string());

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(smtp_host)
            .map_err(|e| AppError::Internal(format!("Failed to create SMTP transport: {}", e)))?
            .credentials(creds)
            .build();

        Ok(Self {
            mailer,
            from_email: from_email.to_string(),
        })
    }

    pub async fn send_order_confirmation(
        &self,
        to_email: &str,
        order: &Order,
        customer_name: &str,
    ) -> AppResult<()> {
        let subject = format!("Order Confirmation - #{}", &order.id.to_string()[..8]);

        let body = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: 'Courier New', monospace; background: #f0e6d2; padding: 20px; }}
        .container {{ max-width: 600px; margin: 0 auto; background: white; padding: 32px; }}
        h1 {{ color: #8b5e3c; font-size: 18px; }}
        .order-id {{ color: #666; font-size: 12px; }}
        .total {{ font-size: 16px; color: #22c55e; margin-top: 20px; }}
        .footer {{ margin-top: 32px; font-size: 10px; color: #888; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Thank you for your order!</h1>
        <p>Hi {},</p>
        <p>We've received your order and are getting it ready for you.</p>
        <p class="order-id">Order ID: {}</p>
        <p class="total">Total: ${:.2}</p>
        <p>We'll send you another email when your order ships.</p>
        <div class="footer">
            <p>Caterpillar Clay - Handmade Pottery</p>
        </div>
    </div>
</body>
</html>"#,
            customer_name,
            order.id,
            order.total_cents as f64 / 100.0
        );

        self.send_email(to_email, &subject, &body).await
    }

    pub async fn send_order_shipped(
        &self,
        to_email: &str,
        order: &Order,
        customer_name: &str,
        tracking_number: &str,
    ) -> AppResult<()> {
        let subject = format!("Your Order Has Shipped - #{}", &order.id.to_string()[..8]);

        let body = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: 'Courier New', monospace; background: #f0e6d2; padding: 20px; }}
        .container {{ max-width: 600px; margin: 0 auto; background: white; padding: 32px; }}
        h1 {{ color: #8b5e3c; font-size: 18px; }}
        .tracking {{ background: #f0fdf4; padding: 16px; margin: 20px 0; font-size: 14px; }}
        .footer {{ margin-top: 32px; font-size: 10px; color: #888; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Your order is on its way!</h1>
        <p>Hi {},</p>
        <p>Great news! Your order has shipped.</p>
        <div class="tracking">
            <strong>Tracking Number:</strong> {}
        </div>
        <p>You can track your package using the tracking number above.</p>
        <div class="footer">
            <p>Caterpillar Clay - Handmade Pottery</p>
        </div>
    </div>
</body>
</html>"#,
            customer_name, tracking_number
        );

        self.send_email(to_email, &subject, &body).await
    }

    pub async fn send_order_delivered(
        &self,
        to_email: &str,
        order: &Order,
        customer_name: &str,
    ) -> AppResult<()> {
        let subject = format!("Your Order Has Been Delivered - #{}", &order.id.to_string()[..8]);

        let body = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: 'Courier New', monospace; background: #f0e6d2; padding: 20px; }}
        .container {{ max-width: 600px; margin: 0 auto; background: white; padding: 32px; }}
        h1 {{ color: #8b5e3c; font-size: 18px; }}
        .footer {{ margin-top: 32px; font-size: 10px; color: #888; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Your order has arrived!</h1>
        <p>Hi {},</p>
        <p>Your Caterpillar Clay order has been delivered!</p>
        <p>We hope you love your new pottery. If you have any questions or concerns, please don't hesitate to reach out.</p>
        <div class="footer">
            <p>Caterpillar Clay - Handmade Pottery</p>
        </div>
    </div>
</body>
</html>"#,
            customer_name
        );

        self.send_email(to_email, &subject, &body).await
    }

    pub async fn send_refund_confirmation(
        &self,
        to_email: &str,
        order: &Order,
        customer_name: &str,
    ) -> AppResult<()> {
        let subject = format!("Refund Processed - #{}", &order.id[..8]);

        let body = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: 'Courier New', monospace; background: #f0e6d2; padding: 20px; }}
        .container {{ max-width: 600px; margin: 0 auto; background: white; padding: 32px; }}
        h1 {{ color: #8b5e3c; font-size: 18px; }}
        .total {{ font-size: 16px; color: #22c55e; margin-top: 20px; }}
        .footer {{ margin-top: 32px; font-size: 10px; color: #888; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Your refund has been processed</h1>
        <p>Hi {},</p>
        <p>We've processed a refund for your order.</p>
        <p class="total">Refund Amount: ${:.2}</p>
        <p>The refund should appear on your statement within 5-10 business days, depending on your bank.</p>
        <p>If you have any questions, please don't hesitate to reach out.</p>
        <div class="footer">
            <p>Caterpillar Clay - Handmade Pottery</p>
        </div>
    </div>
</body>
</html>"#,
            customer_name,
            order.total_cents as f64 / 100.0
        );

        self.send_email(to_email, &subject, &body).await
    }

    async fn send_email(&self, to: &str, subject: &str, html_body: &str) -> AppResult<()> {
        let email = Message::builder()
            .from(
                self.from_email
                    .parse()
                    .map_err(|e| AppError::Internal(format!("Invalid from email: {}", e)))?,
            )
            .to(to
                .parse()
                .map_err(|e| AppError::Internal(format!("Invalid to email: {}", e)))?)
            .subject(subject)
            .header(ContentType::TEXT_HTML)
            .body(html_body.to_string())
            .map_err(|e| AppError::Internal(format!("Failed to build email: {}", e)))?;

        self.mailer
            .send(email)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to send email: {}", e)))?;

        Ok(())
    }
}
