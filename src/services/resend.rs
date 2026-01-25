use resend_rs::types::CreateEmailBaseOptions;
use resend_rs::Resend;

use crate::error::{AppError, AppResult};
use crate::models::Product;

#[derive(Clone)]
pub struct ResendService {
    client: Resend,
    from_email: String,
    base_url: String,
}

impl ResendService {
    pub fn new(api_key: &str, from_email: &str, base_url: &str) -> Self {
        Self {
            client: Resend::new(api_key),
            from_email: from_email.to_string(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn send_welcome_email(&self, to_email: &str, unsubscribe_token: &str) -> AppResult<()> {
        let unsubscribe_url = format!("{}/api/newsletter/unsubscribe?token={}", self.base_url, unsubscribe_token);

        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: 'Courier New', monospace; background: #F8F8F8; padding: 20px; margin: 0; }}
        .container {{ max-width: 600px; margin: 0 auto; background: white; padding: 32px; border: 2px solid #E0E0E0; border-radius: 12px; }}
        h1 {{ color: #97BAD9; font-size: 18px; margin-bottom: 20px; }}
        p {{ color: #18191B; font-size: 14px; line-height: 1.8; }}
        .footer {{ margin-top: 32px; padding-top: 20px; border-top: 1px solid #E0E0E0; font-size: 10px; color: #666; }}
        .footer a {{ color: #97BAD9; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Welcome to Caterpillar Clay!</h1>
        <p>Thank you for subscribing to our newsletter!</p>
        <p>You'll be the first to know when we add new handmade pottery pieces to our shop.</p>
        <p>Each piece is crafted with care, featuring hand-painted designs inspired by local flowers.</p>
        <div class="footer">
            <p>Caterpillar Clay - Handmade Pottery</p>
            <p><a href="{}">Unsubscribe</a></p>
        </div>
    </div>
</body>
</html>"#,
            unsubscribe_url
        );

        self.send_email(to_email, "Welcome to Caterpillar Clay!", &html).await
    }

    pub async fn send_new_product_notification(
        &self,
        to_email: &str,
        unsubscribe_token: &str,
        product: &Product,
        product_image_url: Option<&str>,
    ) -> AppResult<()> {
        let unsubscribe_url = format!("{}/api/newsletter/unsubscribe?token={}", self.base_url, unsubscribe_token);
        let product_url = format!("{}/?product={}", self.base_url, product.id);

        let image_html = if let Some(img_url) = product_image_url {
            format!(r#"<img src="{}" alt="{}" style="max-width:100%;height:auto;border-radius:8px;margin-bottom:20px;border:2px solid #E0E0E0">"#, img_url, product.name)
        } else {
            String::new()
        };

        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: 'Courier New', monospace; background: #F8F8F8; padding: 20px; margin: 0; }}
        .container {{ max-width: 600px; margin: 0 auto; background: white; padding: 32px; border: 2px solid #E0E0E0; border-radius: 12px; text-align: center; }}
        h1 {{ color: #97BAD9; font-size: 16px; margin-bottom: 24px; }}
        h2 {{ color: #18191B; font-size: 14px; margin: 16px 0 8px; }}
        .price {{ color: #97BAD9; font-size: 18px; margin-bottom: 16px; }}
        .description {{ color: #666; font-size: 12px; line-height: 1.8; margin-bottom: 24px; }}
        .btn {{ display: inline-block; background: #97BAD9; color: #18191B; padding: 14px 28px; text-decoration: none; font-size: 12px; border-radius: 8px; font-family: inherit; }}
        .footer {{ margin-top: 32px; padding-top: 20px; border-top: 1px solid #E0E0E0; font-size: 10px; color: #666; }}
        .footer a {{ color: #97BAD9; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>New Arrival!</h1>
        {}
        <h2>{}</h2>
        <p class="price">${:.2}</p>
        <p class="description">{}</p>
        <a href="{}" class="btn">VIEW PRODUCT</a>
        <div class="footer">
            <p>Caterpillar Clay - Handmade Pottery</p>
            <p><a href="{}">Unsubscribe</a></p>
        </div>
    </div>
</body>
</html>"#,
            image_html,
            product.name,
            product.price_cents as f64 / 100.0,
            product.description.as_deref().unwrap_or(""),
            product_url,
            unsubscribe_url
        );

        self.send_email(
            to_email,
            &format!("New Arrival: {} - Caterpillar Clay", product.name),
            &html,
        ).await
    }

    async fn send_email(&self, to: &str, subject: &str, html: &str) -> AppResult<()> {
        let email = CreateEmailBaseOptions::new(&self.from_email, [to], subject)
            .with_html(html);

        self.client
            .emails
            .send(email)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to send email: {}", e)))?;

        Ok(())
    }

    pub async fn send_back_in_stock_notification(
        &self,
        to_email: &str,
        unsubscribe_token: &str,
        product: &Product,
        product_image_url: Option<&str>,
    ) -> AppResult<()> {
        let unsubscribe_url = format!("{}/api/newsletter/unsubscribe?token={}", self.base_url, unsubscribe_token);
        let product_url = format!("{}/?product={}", self.base_url, product.id);

        let image_html = if let Some(img_url) = product_image_url {
            format!(r#"<img src="{}" alt="{}" style="max-width:100%;height:auto;border-radius:8px;margin-bottom:20px;border:2px solid #E0E0E0">"#, img_url, product.name)
        } else {
            String::new()
        };

        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: 'Courier New', monospace; background: #F8F8F8; padding: 20px; margin: 0; }}
        .container {{ max-width: 600px; margin: 0 auto; background: white; padding: 32px; border: 2px solid #E0E0E0; border-radius: 12px; text-align: center; }}
        h1 {{ color: #22c55e; font-size: 16px; margin-bottom: 24px; }}
        h2 {{ color: #18191B; font-size: 14px; margin: 16px 0 8px; }}
        .price {{ color: #97BAD9; font-size: 18px; margin-bottom: 16px; }}
        .description {{ color: #666; font-size: 12px; line-height: 1.8; margin-bottom: 24px; }}
        .btn {{ display: inline-block; background: #22c55e; color: #fff; padding: 14px 28px; text-decoration: none; font-size: 12px; border-radius: 8px; font-family: inherit; }}
        .footer {{ margin-top: 32px; padding-top: 20px; border-top: 1px solid #E0E0E0; font-size: 10px; color: #666; }}
        .footer a {{ color: #97BAD9; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Back in Stock!</h1>
        {}
        <h2>{}</h2>
        <p class="price">${:.2}</p>
        <p class="description">Good news! This item is available again. Grab it before it's gone!</p>
        <a href="{}" class="btn">SHOP NOW</a>
        <div class="footer">
            <p>Caterpillar Clay - Handmade Pottery</p>
            <p><a href="{}">Unsubscribe</a></p>
        </div>
    </div>
</body>
</html>"#,
            image_html,
            product.name,
            product.price_cents as f64 / 100.0,
            product_url,
            unsubscribe_url
        );

        self.send_email(
            to_email,
            &format!("Back in Stock: {} - Caterpillar Clay", product.name),
            &html,
        ).await
    }

    pub async fn send_batch_back_in_stock_notification(
        &self,
        subscribers: &[(String, String)],
        product: &Product,
        product_image_url: Option<&str>,
    ) -> AppResult<usize> {
        let mut sent_count = 0;

        for (email, token) in subscribers {
            if let Err(e) = self.send_back_in_stock_notification(email, token, product, product_image_url).await {
                tracing::error!("Failed to send back in stock notification to {}: {}", email, e);
            } else {
                sent_count += 1;
            }
        }

        Ok(sent_count)
    }

    pub async fn send_batch_new_product_notification(
        &self,
        subscribers: &[(String, String)], // (email, unsubscribe_token)
        product: &Product,
        product_image_url: Option<&str>,
    ) -> AppResult<usize> {
        let mut sent_count = 0;

        for (email, token) in subscribers {
            if let Err(e) = self.send_new_product_notification(email, token, product, product_image_url).await {
                tracing::error!("Failed to send newsletter to {}: {}", email, e);
            } else {
                sent_count += 1;
            }
        }

        Ok(sent_count)
    }

    pub async fn send_batch_multi_product_new(
        &self,
        subscribers: &[(String, String)],
        products: &[(Product, Option<String>)],
    ) -> AppResult<usize> {
        let mut sent_count = 0;

        for (email, token) in subscribers {
            if let Err(e) = self.send_multi_product_new_email(email, token, products).await {
                tracing::error!("Failed to send multi-product new email to {}: {}", email, e);
            } else {
                sent_count += 1;
            }
        }

        Ok(sent_count)
    }

    pub async fn send_batch_multi_product_restock(
        &self,
        subscribers: &[(String, String)],
        products: &[(Product, Option<String>)],
    ) -> AppResult<usize> {
        let mut sent_count = 0;

        for (email, token) in subscribers {
            if let Err(e) = self.send_multi_product_restock_email(email, token, products).await {
                tracing::error!("Failed to send multi-product restock email to {}: {}", email, e);
            } else {
                sent_count += 1;
            }
        }

        Ok(sent_count)
    }

    async fn send_multi_product_new_email(
        &self,
        to_email: &str,
        unsubscribe_token: &str,
        products: &[(Product, Option<String>)],
    ) -> AppResult<()> {
        let unsubscribe_url = format!("{}/api/newsletter/unsubscribe?token={}", self.base_url, unsubscribe_token);

        let products_html: String = products.iter().map(|(product, image_url)| {
            let product_url = format!("{}/?product={}", self.base_url, product.id);
            let image_html = if let Some(img_url) = image_url {
                format!(r#"<img src="{}" alt="{}" style="width:120px;height:120px;object-fit:cover;border-radius:8px;border:2px solid #E0E0E0">"#, img_url, product.name)
            } else {
                String::from(r#"<div style="width:120px;height:120px;background:#E0E0E0;border-radius:8px"></div>"#)
            };
            format!(
                r#"<a href="{}" style="display:inline-block;text-align:center;margin:8px;text-decoration:none;color:#18191B">
                    {}
                    <p style="font-size:10px;margin:8px 0 4px;font-family:'Courier New',monospace">{}</p>
                    <p style="font-size:12px;color:#97BAD9;font-family:'Courier New',monospace">${:.2}</p>
                </a>"#,
                product_url, image_html, product.name, product.price_cents as f64 / 100.0
            )
        }).collect();

        let subject = if products.len() == 1 {
            format!("New Arrival: {} - Caterpillar Clay", products[0].0.name)
        } else {
            format!("{} New Arrivals - Caterpillar Clay", products.len())
        };

        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: 'Courier New', monospace; background: #F8F8F8; padding: 20px; margin: 0; }}
        .container {{ max-width: 600px; margin: 0 auto; background: white; padding: 32px; border: 2px solid #E0E0E0; border-radius: 12px; text-align: center; }}
        h1 {{ color: #97BAD9; font-size: 16px; margin-bottom: 24px; }}
        .products {{ margin: 24px 0; }}
        .btn {{ display: inline-block; background: #97BAD9; color: #18191B; padding: 14px 28px; text-decoration: none; font-size: 12px; border-radius: 8px; font-family: inherit; }}
        .footer {{ margin-top: 32px; padding-top: 20px; border-top: 1px solid #E0E0E0; font-size: 10px; color: #666; }}
        .footer a {{ color: #97BAD9; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>New Arrivals!</h1>
        <p style="font-size:12px;color:#666;margin-bottom:24px">Check out our latest handmade pottery pieces</p>
        <div class="products">{}</div>
        <a href="{}" class="btn">SHOP NOW</a>
        <div class="footer">
            <p>Caterpillar Clay - Handmade Pottery</p>
            <p><a href="{}">Unsubscribe</a></p>
        </div>
    </div>
</body>
</html>"#,
            products_html,
            self.base_url,
            unsubscribe_url
        );

        self.send_email(to_email, &subject, &html).await
    }

    async fn send_multi_product_restock_email(
        &self,
        to_email: &str,
        unsubscribe_token: &str,
        products: &[(Product, Option<String>)],
    ) -> AppResult<()> {
        let unsubscribe_url = format!("{}/api/newsletter/unsubscribe?token={}", self.base_url, unsubscribe_token);

        let products_html: String = products.iter().map(|(product, image_url)| {
            let product_url = format!("{}/?product={}", self.base_url, product.id);
            let image_html = if let Some(img_url) = image_url {
                format!(r#"<img src="{}" alt="{}" style="width:120px;height:120px;object-fit:cover;border-radius:8px;border:2px solid #E0E0E0">"#, img_url, product.name)
            } else {
                String::from(r#"<div style="width:120px;height:120px;background:#E0E0E0;border-radius:8px"></div>"#)
            };
            format!(
                r#"<a href="{}" style="display:inline-block;text-align:center;margin:8px;text-decoration:none;color:#18191B">
                    {}
                    <p style="font-size:10px;margin:8px 0 4px;font-family:'Courier New',monospace">{}</p>
                    <p style="font-size:12px;color:#97BAD9;font-family:'Courier New',monospace">${:.2}</p>
                </a>"#,
                product_url, image_html, product.name, product.price_cents as f64 / 100.0
            )
        }).collect();

        let subject = if products.len() == 1 {
            format!("Back in Stock: {} - Caterpillar Clay", products[0].0.name)
        } else {
            format!("{} Items Back in Stock - Caterpillar Clay", products.len())
        };

        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: 'Courier New', monospace; background: #F8F8F8; padding: 20px; margin: 0; }}
        .container {{ max-width: 600px; margin: 0 auto; background: white; padding: 32px; border: 2px solid #E0E0E0; border-radius: 12px; text-align: center; }}
        h1 {{ color: #22c55e; font-size: 16px; margin-bottom: 24px; }}
        .products {{ margin: 24px 0; }}
        .btn {{ display: inline-block; background: #22c55e; color: #fff; padding: 14px 28px; text-decoration: none; font-size: 12px; border-radius: 8px; font-family: inherit; }}
        .footer {{ margin-top: 32px; padding-top: 20px; border-top: 1px solid #E0E0E0; font-size: 10px; color: #666; }}
        .footer a {{ color: #97BAD9; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Back in Stock!</h1>
        <p style="font-size:12px;color:#666;margin-bottom:24px">Good news! These items are available again</p>
        <div class="products">{}</div>
        <a href="{}" class="btn">SHOP NOW</a>
        <div class="footer">
            <p>Caterpillar Clay - Handmade Pottery</p>
            <p><a href="{}">Unsubscribe</a></p>
        </div>
    </div>
</body>
</html>"#,
            products_html,
            self.base_url,
            unsubscribe_url
        );

        self.send_email(to_email, &subject, &html).await
    }
}
