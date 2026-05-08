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
    
    // Check seller_rating before
    println!("Before approval:");
    let row = sqlx::query("SELECT seller_account_id, display_name, seller_rating::text as seller_rating FROM seller_accounts WHERE owner_id = 'bench-seller'")
        .fetch_one(&pool)
        .await?;
    let seller_id: String = row.get("seller_account_id");
    let display_name: String = row.get("display_name");
    let rating: Option<String> = row.get("seller_rating");
    println!("  Seller: {} (id: {})", display_name, seller_id);
    println!("  seller_rating: {:?}", rating);
    
    // Approve the review
    println!("\nApproving review...");
    sqlx::query("UPDATE reviews SET status = 'approved' WHERE review_id = 'rev_57f11189-cc84-4b74-82f0-0ec814a5c4f4'")
        .execute(&pool)
        .await?;
    println!("  Review approved!");
    
    // Check seller_rating after (should be auto-updated by trigger)
    println!("\nAfter approval:");
    let row = sqlx::query("SELECT seller_rating::text as seller_rating FROM seller_accounts WHERE seller_account_id = $1")
        .bind(&seller_id)
        .fetch_one(&pool)
        .await?;
    let rating: Option<String> = row.get("seller_rating");
    println!("  seller_rating: {:?}", rating);
    
    // List all reviews for this seller
    println!("\nAll reviews for seller:");
    let rows = sqlx::query("SELECT review_id, rating, status FROM reviews WHERE seller_account_id = $1")
        .bind(&seller_id)
        .fetch_all(&pool)
        .await?;
    for row in rows {
        let id: String = row.get("review_id");
        let rating: i32 = row.get("rating");
        let status: String = row.get("status");
        println!("  {}: rating={}, status={}", id, rating, status);
    }
    
    Ok(())
}
