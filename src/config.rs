use std::env;

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub clerk_secret_key: String,
    pub clerk_publishable_key: String,
    pub polar_access_token: String,
    pub polar_webhook_secret: String,
    pub easypost_api_key: String,
    pub easypost_webhook_secret: String,
    pub smtp_host: String,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub from_email: String,
    pub resend_api_key: Option<String>,
    pub storage_type: String,
    pub upload_dir: String,
    pub r2_bucket: Option<String>,
    pub r2_account_id: Option<String>,
    pub r2_access_key: Option<String>,
    pub r2_secret_key: Option<String>,
    pub r2_public_url: Option<String>,
    pub base_url: String,
    pub port: u16,
    pub testing_mode: bool,
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        Ok(Self {
            database_url: env::var("DATABASE_URL")?,
            clerk_secret_key: env::var("CLERK_SECRET_KEY")?,
            clerk_publishable_key: env::var("CLERK_PUBLISHABLE_KEY")?,
            polar_access_token: env::var("POLAR_ACCESS_TOKEN")?,
            polar_webhook_secret: env::var("POLAR_WEBHOOK_SECRET")?,
            easypost_api_key: env::var("EASYPOST_API_KEY")?,
            easypost_webhook_secret: env::var("EASYPOST_WEBHOOK_SECRET")?,
            smtp_host: env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.resend.com".to_string()),
            smtp_user: env::var("SMTP_USER").unwrap_or_else(|_| "resend".to_string()),
            smtp_pass: env::var("SMTP_PASS")?,
            from_email: env::var("FROM_EMAIL")
                .unwrap_or_else(|_| "CaterpillarClay@caterpillarclay.com".to_string()),
            resend_api_key: env::var("RESEND_API_KEY").ok(),
            storage_type: env::var("STORAGE_TYPE").unwrap_or_else(|_| "local".to_string()),
            upload_dir: env::var("UPLOAD_DIR").unwrap_or_else(|_| "./static/uploads".to_string()),
            r2_bucket: env::var("R2_BUCKET").ok(),
            r2_account_id: env::var("R2_ACCOUNT_ID").ok(),
            r2_access_key: env::var("R2_ACCESS_KEY").ok(),
            r2_secret_key: env::var("R2_SECRET_KEY").ok(),
            r2_public_url: env::var("R2_PUBLIC_URL").ok(),
            base_url: env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .unwrap_or(3000),
            testing_mode: env::var("TESTING_MODE")
                .unwrap_or_else(|_| "false".to_string())
                .to_lowercase() == "true",
        })
    }
}
