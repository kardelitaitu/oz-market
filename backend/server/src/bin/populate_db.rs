use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::env;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable".to_string());
    
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    
    println!("🌱 Starting database population...");
    
    // Configuration
    let num_sellers = 1000;
    let num_listings_per_seller = 100; // Total: 100,000 listings
    let num_reviews_per_listing = 3;
    
    // 1. Create seller accounts (batch)
    println!("\n📝 Creating {} seller accounts...", num_sellers);
    for i in 0..num_sellers {
        let seller_account_id = Uuid::new_v4().to_string();
        let owner_id = format!("seller_{}", i);
        let display_name = format!("Seller {} Shop", i);
        let trust_levels = ["new", "verified", "trusted", "restricted"];
        let trust_level = trust_levels[i % 4];
        
        sqlx::query(
            "INSERT INTO seller_accounts (seller_account_id, owner_id, display_name, trust_level, status, listings_created)
             VALUES ($1, $2, $3, $4, 'active', 0)"
        )
        .bind(&seller_account_id)
        .bind(&owner_id)
        .bind(&display_name)
        .bind(trust_level)
        .execute(&pool)
        .await?;
        
        if (i + 1) % 100 == 0 {
            println!("  Created {}/{} sellers...", i + 1, num_sellers);
        }
    }
    println!("✅ Created {} seller accounts", num_sellers);
    
    // 2. Create listings (batch per seller)
    println!("\n📦 Creating {} listings ({} per seller)...", num_sellers * num_listings_per_seller, num_listings_per_seller);
    let categories = ["laptop", "phone", "tablet", "desktop", "monitor", "accessory", "camera", "audio", "gaming", "appliance"];
    let conditions = ["new", "used", "refurbished"];
    let cities = [("JP", "Japan", "Tokyo"), ("US", "United States", "New York"), ("UK", "United Kingdom", "London")];
    let brands = ["Apple", "Samsung", "Dell", "HP", "Lenovo", "Asus", "Acer", "Sony", "LG", "Bose"];
    
    for seller_idx in 0..num_sellers {
        let owner_id = format!("seller_{}", seller_idx);
        
        for j in 0..num_listings_per_seller {
            let listing_id = format!("lst_{:07}", seller_idx * num_listings_per_seller + j + 1);
            let category = categories[(seller_idx + j) % categories.len()];
            let condition = conditions[(seller_idx + j) % conditions.len()];
            let city_data = &cities[(seller_idx + j) % cities.len()];
            let brand = brands[(seller_idx + j) % brands.len()];
            let price = 100.0 + ((seller_idx * num_listings_per_seller + j) as f64 % 1000.0);
            
            let product_name = format!("{} {} {}", brand, category.to_uppercase(), j + 1);
            let description = format!("High-quality {} from {}. Excellent condition, barely used.", category, brand);
            
            sqlx::query(
                "INSERT INTO listings (
                    listing_id, owner_id, schema_version, category, product_name, 
                    condition, price_currency, price_amount, 
                    country_code, country_name, city,
                    picture_urls, description, attributes, status, version, create_idempotency_key, search_text
                ) VALUES (
                    $1, $2, '1.0', $3, $4, $5, 'USD', $6,
                    $7, $8, $9, $10, $11, $12, 'active', 1, $13, $14
                )"
            )
            .bind(&listing_id)
            .bind(&owner_id)
            .bind(category)
            .bind(&product_name)
            .bind(condition)
            .bind(price)
            .bind(city_data.0)
            .bind(city_data.1)
            .bind(city_data.2)
            .bind(serde_json::json!([format!("https://example.com/{}.jpg", listing_id)]))
            .bind(&description)
            .bind(&serde_json::json!({"brand": brand, "model": format!("{}-{}", category, j + 1)}))
            .bind(&Uuid::new_v4().to_string())
            .bind(format!("{} {} {} {}", category, product_name, description, city_data.2))
            .execute(&pool)
            .await?;
        }
        
        if (seller_idx + 1) % 100 == 0 {
            println!("  Created listings for {}/{} sellers...", seller_idx + 1, num_sellers);
        }
    }
    println!("✅ Created {} listings", num_sellers * num_listings_per_seller);
    
    // 3. Create reviews
    println!("\n⭐ Creating reviews (avg {} per listing)...", num_reviews_per_listing);
    let review_titles = ["Great product!", "Excellent seller", "Fast shipping", "Good quality", "Would buy again"];
    let review_bodies = ["Really happy with this purchase.", "Seller was very professional.", "Item arrived quickly."];
    
    for seller_idx in 0..num_sellers {
        let owner_id = format!("seller_{}", seller_idx);
        
        // Get seller_account_id
        let row = sqlx::query("SELECT seller_account_id FROM seller_accounts WHERE owner_id = $1")
            .bind(&owner_id)
            .fetch_one(&pool)
            .await?;
        let seller_account_id: String = row.get("seller_account_id");
        
        for j in 0..num_listings_per_seller {
            let listing_id = format!("lst_{:07}", seller_idx * num_listings_per_seller + j + 1);
            
            for k in 0..num_reviews_per_listing {
                let review_id = Uuid::new_v4().to_string();
                let reviewer_id = format!("buyer_{}_{}", listing_id, k);
                let rating: i32 = 3 + (k % 3) as i32; // Ratings 3-5
                let title = review_titles[(seller_idx + j + k) % review_titles.len()];
                let body = review_bodies[(seller_idx + j + k) % review_bodies.len()];
                let status = if k == 0 { "approved" } else { "pending" };
                
                sqlx::query(
                    "INSERT INTO reviews (review_id, listing_id, seller_account_id, reviewer_id, rating, title, body, status)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
                )
                .bind(&review_id)
                .bind(&listing_id)
                .bind(&seller_account_id)
                .bind(&reviewer_id)
                .bind(rating)
                .bind(title)
                .bind(body)
                .bind(status)
                .execute(&pool)
                .await?;
            }
        }
        
        if (seller_idx + 1) % 100 == 0 {
            println!("  Created reviews for {}/{} sellers...", seller_idx + 1, num_sellers);
        }
    }
    println!("✅ Created {} reviews", num_sellers * num_listings_per_seller * num_reviews_per_listing);
    
    // 4. Update seller_rating for sellers with approved reviews
    println!("\n📊 Updating seller ratings...");
    let updated = sqlx::query(
        "UPDATE seller_accounts sa SET seller_rating = (
            SELECT ROUND(AVG(r.rating)::numeric, 2)::DECIMAL(3,2)
            FROM reviews r 
            WHERE r.seller_account_id = sa.seller_account_id AND r.status = 'approved'
        ) WHERE EXISTS (
            SELECT 1 FROM reviews r2 
            WHERE r2.seller_account_id = sa.seller_account_id AND r2.status = 'approved'
        )"
    )
    .execute(&pool)
    .await?;
    println!("✅ Updated ratings for {} sellers", updated.rows_affected());
    
    // 5. Update listings_created count
    println!("\n🔢 Updating listings count...");
    let updated = sqlx::query(
        "UPDATE seller_accounts sa SET listings_created = (
            SELECT COUNT(*) FROM listings l WHERE l.owner_id = sa.owner_id
        )"
    )
    .execute(&pool)
    .await?;
    println!("✅ Updated listings count for {} sellers", updated.rows_affected());
    
    println!("\n🎉 Database population complete!");
    println!("📈 Summary:");
    println!("  - {} sellers", num_sellers);
    println!("  - {} listings", num_sellers * num_listings_per_seller);
    println!("  - {} reviews", num_sellers * num_listings_per_seller * num_reviews_per_listing);
    println!("\n💡 Tip: Run benchmark again with populated database!");
    
    Ok(())
}
