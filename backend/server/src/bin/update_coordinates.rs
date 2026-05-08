use sqlx::{postgres::PgPoolOptions, Row};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable".to_string()
    });

    println!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("Adding random coordinates to 10000 listings...");
    let result = sqlx::query(
        r#"
        UPDATE listings
        SET latitude = 40.7128 + (random() - 0.5) * 0.5,
            longitude = -74.0060 + (random() - 0.5) * 0.5
        WHERE listing_id IN (
            SELECT listing_id FROM listings
            WHERE latitude IS NULL
            LIMIT 10000
        )
        "#,
    )
    .execute(&pool)
    .await?;

    println!("Updated {} rows with coordinates", result.rows_affected());

    println!("Setting 2000 random listings to opt out...");
    let result2 = sqlx::query(
        r#"
        UPDATE listings
        SET geolocation_opt_out = true
        WHERE listing_id IN (
            SELECT listing_id FROM listings
            WHERE latitude IS NOT NULL
            ORDER BY random()
            LIMIT 2000
        )
        "#,
    )
    .execute(&pool)
    .await?;

    println!("Set {} listings to opt out", result2.rows_affected());

    println!("\nStatistics:");
    let row = sqlx::query(
        r#"
        SELECT 
            COUNT(*) as total,
            COUNT(latitude) as with_coords,
            COUNT(*) FILTER (WHERE geolocation_opt_out = true) as opted_out,
            COUNT(*) FILTER (WHERE latitude IS NOT NULL AND (geolocation_opt_out IS NULL OR geolocation_opt_out = false)) as available_for_near_me
        FROM listings
        "#
    )
    .fetch_one(&pool)
    .await?;

    let total: i64 = row.get("total");
    let with_coords: i64 = row.get("with_coords");
    let opted_out: i64 = row.get("opted_out");
    let available: i64 = row.get("available_for_near_me");

    println!("Total listings: {}", total);
    println!("With coordinates: {}", with_coords);
    println!("Opted out: {}", opted_out);
    println!("Available for near_me: {}", available);

    println!("\nDone!");
    Ok(())
}
