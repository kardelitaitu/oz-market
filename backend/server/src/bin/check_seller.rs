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
    
    // Check seller_accounts
    println!("Seller accounts:");
    let rows = sqlx::query("SELECT seller_account_id, owner_id, display_name FROM seller_accounts LIMIT 10")
        .fetch_all(&pool)
        .await?;
    
    for row in rows {
        let id: String = row.get("seller_account_id");
        let owner_id: String = row.get("owner_id");
        let display_name: Option<String> = row.get("display_name");
        println!("  {} (owner: {}) - display_name: {:?}", id, owner_id, display_name);
    }
    
    // Check listings and their owners
    println!("\nListings (first 5):");
    let rows = sqlx::query("SELECT listing_id, owner_id FROM listings LIMIT 5")
        .fetch_all(&pool)
        .await?;
    
    for row in rows {
        let id: String = row.get("listing_id");
        let owner_id: String = row.get("owner_id");
        println!("  {} (owner: {})", id, owner_id);
    }
    
    Ok(())
}
