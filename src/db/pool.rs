use libsql::{Builder, Database};

pub async fn create_database(database_url: &str) -> Result<Database, libsql::Error> {
    // Parse the database URL - libsql uses file path or Turso URL
    let path = database_url
        .strip_prefix("sqlite:")
        .unwrap_or(database_url)
        .split('?')
        .next()
        .unwrap_or("caterpillar_clay.db");

    Builder::new_local(path).build().await
}
