use sqlx::postgres::PgPoolOptions;
use std::env;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable".to_string());
    
    println!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;
    
    // Drop existing triggers first
    println!("Dropping existing triggers if any...");
    let _ = sqlx::query("DROP TRIGGER IF EXISTS update_seller_rating_on_insert ON reviews")
        .execute(&pool)
        .await;
    let _ = sqlx::query("DROP TRIGGER IF EXISTS update_seller_rating_on_update ON reviews")
        .execute(&pool)
        .await;
    let _ = sqlx::query("DROP TRIGGER IF EXISTS update_seller_rating_on_delete ON reviews")
        .execute(&pool)
        .await;
    let _ = sqlx::query("DROP FUNCTION IF EXISTS update_seller_rating()")
        .execute(&pool)
        .await;
    
    // Create the function using a DO block approach
    println!("Creating update_seller_rating function...");
    let create_function = r#"
CREATE OR REPLACE FUNCTION update_seller_rating()
RETURNS TRIGGER AS $$
DECLARE
    avg_rating DECIMAL(3,2);
BEGIN
    IF TG_OP = 'DELETE' THEN
        SELECT ROUND(AVG(rating)::numeric, 2)::DECIMAL(3,2)
        INTO avg_rating
        FROM reviews
        WHERE seller_account_id = OLD.seller_account_id AND status = 'approved';
    ELSE
        SELECT ROUND(AVG(rating)::numeric, 2)::DECIMAL(3,2)
        INTO avg_rating
        FROM reviews
        WHERE seller_account_id = NEW.seller_account_id AND status = 'approved';
    END IF;
    
    IF TG_OP = 'DELETE' THEN
        UPDATE seller_accounts
        SET seller_rating = avg_rating
        WHERE seller_account_id = OLD.seller_account_id;
        RETURN OLD;
    ELSE
        UPDATE seller_accounts
        SET seller_rating = avg_rating
        WHERE seller_account_id = NEW.seller_account_id;
        RETURN NEW;
    END IF;
END;
$$ LANGUAGE plpgsql;
"#;
    
    match sqlx::query(create_function).execute(&pool).await {
        Ok(_) => println!("  Function created successfully!"),
        Err(e) => {
            eprintln!("  Error creating function: {}", e);
            return Err(e);
        }
    }
    
    // Create triggers
    println!("Creating triggers...");
    
    let trigger_insert = r#"
CREATE TRIGGER update_seller_rating_on_insert
AFTER INSERT ON reviews
FOR EACH ROW
WHEN (NEW.status = 'approved')
EXECUTE FUNCTION update_seller_rating();
"#;
    
    match sqlx::query(trigger_insert).execute(&pool).await {
        Ok(_) => println!("  Trigger INSERT created!"),
        Err(e) => eprintln!("  Error creating INSERT trigger: {}", e),
    }
    
    let trigger_update = r#"
CREATE TRIGGER update_seller_rating_on_update
AFTER UPDATE ON reviews
FOR EACH ROW
WHEN (OLD.status != 'approved' AND NEW.status = 'approved' OR OLD.status = 'approved' AND NEW.status != 'approved')
EXECUTE FUNCTION update_seller_rating();
"#;
    
    match sqlx::query(trigger_update).execute(&pool).await {
        Ok(_) => println!("  Trigger UPDATE created!"),
        Err(e) => eprintln!("  Error creating UPDATE trigger: {}", e),
    }
    
    let trigger_delete = r#"
CREATE TRIGGER update_seller_rating_on_delete
AFTER DELETE ON reviews
FOR EACH ROW
EXECUTE FUNCTION update_seller_rating();
"#;
    
    match sqlx::query(trigger_delete).execute(&pool).await {
        Ok(_) => println!("  Trigger DELETE created!"),
        Err(e) => eprintln!("  Error creating DELETE trigger: {}", e),
    }
    
    println!("\nAll triggers applied successfully!");
    Ok(())
}
