use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::env;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable".to_string()
    });

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;

    // Check counts
    let row = sqlx::query("SELECT COUNT(*) as count FROM seller_accounts")
        .fetch_one(&pool)
        .await?;
    let sellers: i64 = row.get("count");
    println!("Sellers: {}", sellers);

    let row = sqlx::query("SELECT COUNT(*) as count FROM listings")
        .fetch_one(&pool)
        .await?;
    let listings: i64 = row.get("count");
    println!("Listings: {}", listings);

    let row = sqlx::query("SELECT COUNT(*) as count FROM reviews")
        .fetch_one(&pool)
        .await?;
    let reviews: i64 = row.get("count");
    println!("Reviews: {}", reviews);

    // Check a few listings
    println!("\nFirst 3 listings:");
    let rows = sqlx::query("SELECT listing_id, owner_id, product_name FROM listings LIMIT 3")
        .fetch_all(&pool)
        .await?;
    for row in rows {
        let id: String = row.get("listing_id");
        let owner: String = row.get("owner_id");
        let name: String = row.get("product_name");
        println!("  {} (owner: {}) - {}", id, owner, name);
    }

    Ok(())
}
