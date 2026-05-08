use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::env;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable".to_string());
    
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;
    
    // Get distinct owners from listings that don't have a seller_account
    let rows = sqlx::query(
        "SELECT DISTINCT l.owner_id FROM listings l 
         WHERE NOT EXISTS (
             SELECT 1 FROM seller_accounts s WHERE s.owner_id = l.owner_id
         )"
    )
    .fetch_all(&pool)
    .await?;
    
    println!("Creating seller accounts for {} owners...", rows.len());
    
    for row in rows {
        let owner_id: String = row.get("owner_id");
        let seller_account_id = uuid::Uuid::new_v4().to_string();
        let display_name = format!("Seller {}", &owner_id);
        
        let result = sqlx::query(
            "INSERT INTO seller_accounts (seller_account_id, owner_id, display_name, trust_level, status, seller_rating) 
             VALUES ($1, $2, $3, 'new', 'active', NULL)"
        )
        .bind(&seller_account_id)
        .bind(&owner_id)
        .bind(&display_name)
        .execute(&pool)
        .await;
        
        match result {
            Ok(_) => println!("  Created seller account for owner: {} (id: {})", owner_id, seller_account_id),
            Err(e) => eprintln!("  Error creating seller account for {}: {}", owner_id, e),
        }
    }
    
    println!("\nDone!");
    Ok(())
}
