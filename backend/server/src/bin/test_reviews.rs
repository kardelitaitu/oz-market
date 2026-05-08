use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::env;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable".to_string());
    
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;
    
    let listing_id = "lst_000001";
    println!("Querying reviews for listing: {}", listing_id);
    
    let rows = sqlx::query(
        "SELECT review_id, listing_id, seller_account_id, reviewer_id, rating, title, body, status
         FROM reviews WHERE listing_id = $1 ORDER BY created_at DESC"
    )
    .bind(listing_id)
    .fetch_all(&pool)
    .await?;
    
    println!("Found {} reviews", rows.len());
    for row in rows {
        let review_id: String = row.get("review_id");
        let rating: i32 = row.get("rating");
        let title: String = row.get("title");
        println!("  {}: rating={}, title={}", review_id, rating, title);
    }
    
    Ok(())
}
