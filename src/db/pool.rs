use libsql::{Builder, Database};
use std::env;

pub async fn create_database(database_url: &str) -> Result<Database, libsql::Error> {
    // Check if this is a Turso remote URL
    if database_url.starts_with("libsql://") {
        let auth_token = env::var("TURSO_AUTH_TOKEN")
            .expect("TURSO_AUTH_TOKEN must be set for remote database");

        Builder::new_remote(database_url.to_string(), auth_token)
            .build()
            .await
    } else {
        // Local SQLite file
        let path = database_url
            .strip_prefix("sqlite:")
            .unwrap_or(database_url)
            .split('?')
            .next()
            .unwrap_or("caterpillar_clay.db");

        Builder::new_local(path).build().await
    }
}
