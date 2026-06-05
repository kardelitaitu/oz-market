use sqlx::postgres::PgPoolOptions;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable".to_string()
    });

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;

    oz_market_server::bootstrap::apply_schema(&pool).await?;
    println!("schema bootstrap complete");

    Ok(())
}
