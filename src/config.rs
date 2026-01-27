use std::env;

#[derive(Clone, PartialEq)]
pub enum DeployMode {
    Local,
    Cloud,
}

impl DeployMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cloud" | "production" | "prod" => DeployMode::Cloud,
            _ => DeployMode::Local,
        }
    }

    pub fn is_cloud(&self) -> bool {
        matches!(self, DeployMode::Cloud)
    }
}

#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub turso_auth_token: Option<String>,
    pub clerk_secret_key: String,
    pub clerk_publishable_key: String,
    pub clerk_jwks_url: String,
    pub stripe_secret_key: String,
    pub stripe_publishable_key: String,
    pub stripe_webhook_secret: String,
    pub shippo_api_key: String,
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
    pub deploy_mode: DeployMode,
    // Rate limiting (requests per minute)
    pub rate_limit_general: u32,
    pub rate_limit_auth: u32,
    pub rate_limit_checkout: u32,
    // Upstash Redis (for distributed rate limiting)
    // Format: rediss://default:TOKEN@host:6379
    pub upstash_redis_url: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        // Read testing mode first to determine which keys to use
        let testing_mode = env::var("TESTING_MODE")
            .unwrap_or_else(|_| "false".to_string())
            .to_lowercase() == "true";

        // Deploy mode: local (development) vs cloud (production with webhooks)
        let deploy_mode = DeployMode::from_str(
            &env::var("DEPLOY_MODE").unwrap_or_else(|_| "local".to_string())
        );

        let suffix = if testing_mode { "_TEST" } else { "_PROD" };

        // Helper to get env var with test/prod suffix, falling back to non-suffixed
        let get_env = |key: &str| -> Result<String, env::VarError> {
            env::var(format!("{}{}", key, suffix))
                .or_else(|_| env::var(key))
        };

        let get_env_optional = |key: &str| -> Option<String> {
            env::var(format!("{}{}", key, suffix))
                .or_else(|_| env::var(key))
                .ok()
        };

        let clerk_publishable_key = get_env("CLERK_PUBLISHABLE_KEY")?;

        // JWKS URL - must be set in env (derived from Clerk frontend API domain)
        let clerk_jwks_url = env::var("CLERK_JWKS_URL")
            .expect("CLERK_JWKS_URL must be set (e.g., https://your-app.clerk.accounts.dev/.well-known/jwks.json)");

        Ok(Self {
            database_url: get_env("DATABASE_URL")?,
            turso_auth_token: get_env_optional("TURSO_AUTH_TOKEN"),
            clerk_secret_key: get_env("CLERK_SECRET_KEY")?,
            clerk_publishable_key,
            clerk_jwks_url,
            stripe_secret_key: get_env("STRIPE_SECRET_KEY")?,
            stripe_publishable_key: get_env_optional("STRIPE_PUBLISHABLE_KEY")
                .unwrap_or_default(),
            stripe_webhook_secret: {
                // Webhook secret depends on both testing_mode and deploy_mode
                // TEST_LOCAL, TEST_CLOUD, or PROD
                let deploy_suffix = match &deploy_mode {
                    DeployMode::Local => "_LOCAL",
                    DeployMode::Cloud => "_CLOUD",
                };
                if testing_mode {
                    env::var(format!("STRIPE_WEBHOOK_SECRET_TEST{}", deploy_suffix))
                        .or_else(|_| env::var("STRIPE_WEBHOOK_SECRET_TEST"))
                        .unwrap_or_default()
                } else {
                    // Production only uses cloud (no local suffix for prod)
                    env::var("STRIPE_WEBHOOK_SECRET_PROD")
                        .unwrap_or_default()
                }
            },
            shippo_api_key: get_env("SHIPPO_API_KEY")?,
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
            testing_mode,
            deploy_mode,
            rate_limit_general: env::var("RATE_LIMIT_GENERAL")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
            rate_limit_auth: env::var("RATE_LIMIT_AUTH")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
            rate_limit_checkout: env::var("RATE_LIMIT_CHECKOUT")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
            upstash_redis_url: env::var("UPSTASH_REDIS_URL").ok(),
        })
    }
}
