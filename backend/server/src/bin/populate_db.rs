use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use std::env;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://marketplace:marketplace@localhost:5432/marketplace?sslmode=disable".to_string()
    });

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("🌱 Starting database population...");

    let num_sellers = env::var("POPULATE_NUM_SELLERS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1000);
    let num_listings_per_seller = env::var("POPULATE_LISTINGS_PER_SELLER")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(100);
    let reviews_per_listing = env::var("POPULATE_REVIEWS_PER_LISTING")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1);

    let product_categories = [
        "laptop",
        "phone",
        "tablet",
        "desktop",
        "monitor",
        "accessory",
        "camera",
        "audio",
        "gaming",
        "appliance",
        "furniture",
        "vehicle_part",
        "other",
    ];
    let conditions = ["new", "used", "refurbished"];
    let cities = [
        ("JP", "Japan", "Tokyo"),
        ("US", "United States", "New York"),
        ("UK", "United Kingdom", "London"),
    ];
    let brands = [
        "Apple", "Samsung", "Dell", "HP", "Lenovo", "Asus", "Acer", "Sony", "LG", "Bose",
    ];

    println!("\n📝 Creating {} seller accounts...", num_sellers);
    for seller_idx in 0..num_sellers {
        let seller_account_id = Uuid::new_v4().to_string();
        let owner_id = format!("seller_{}", seller_idx);
        let display_name = format!("Seller {} Shop", seller_idx);
        let trust_level = match seller_idx % 4 {
            0 => "new",
            1 => "verified",
            2 => "trusted",
            _ => "restricted",
        };

        sqlx::query(
            "INSERT INTO seller_accounts (seller_account_id, owner_id, display_name, trust_level, status, seller_rating, listings_created)
             VALUES ($1, $2, $3, $4, 'active', 4.50, 0)"
        )
        .bind(&seller_account_id)
        .bind(&owner_id)
        .bind(&display_name)
        .bind(trust_level)
        .execute(&pool)
        .await?;

        if (seller_idx + 1) % 100 == 0 {
            println!("  Created {}/{} sellers...", seller_idx + 1, num_sellers);
        }
    }
    println!("✅ Created {} seller accounts", num_sellers);

    println!(
        "\n📦 Creating {} listings ({} per seller)...",
        num_sellers * num_listings_per_seller,
        num_listings_per_seller
    );

    let mut listing_count = 0usize;
    for seller_idx in 0..num_sellers {
        let owner_id = format!("seller_{}", seller_idx);

        for item_idx in 0..num_listings_per_seller {
            let global_idx = seller_idx * num_listings_per_seller + item_idx + 1;
            let listing_id = format!("lst_{global_idx:07}");
            let city_data = &cities[(seller_idx + item_idx) % cities.len()];
            let price = 100.0 + (global_idx as f64 % 2500.0);

            let (
                listing_type,
                category,
                condition,
                title,
                description,
                sku,
                quantity,
                shipping_info,
                condition_details,
                seller_notes,
                service_type,
                hourly_rate,
                project_rate,
                qualifications,
                service_radius_km,
                property_transaction_type,
                property_sub_type,
                area_sqm,
                bedrooms,
                bathrooms,
                year_built,
                lot_size_sqm,
                zoning,
            ) = match global_idx % 5 {
                0 => {
                    let brand = brands[(seller_idx + item_idx) % brands.len()];
                    let category = product_categories[(seller_idx + item_idx) % 10];
                    let condition = conditions[(seller_idx + item_idx) % conditions.len()];
                    let title = format!("{} {} {}", brand, category.to_uppercase(), item_idx + 1);
                    let description = format!(
                        "High-quality {} from {}. Excellent condition, barely used.",
                        category, brand
                    );
                    (
                        "product",
                        category,
                        condition,
                        title,
                        description,
                        Some(format!("SKU-{global_idx}")),
                        Some(1_i32),
                        Some(serde_json::json!({
                            "local_pickup": true,
                            "shipping_available": true,
                            "shipping_cost": {"currency": "USD", "amount": 15.99},
                            "shipping_regions": ["US", "CA"],
                        })),
                        Some("Excellent condition, lightly used".to_string()),
                        if global_idx.is_multiple_of(10) {
                            Some("Special discount available!".to_string())
                        } else {
                            None
                        },
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                }
                1 | 2 => {
                    let title = format!("Math Tutoring Service {}", global_idx);
                    let description =
                        "Online and local tutoring service for mathematics.".to_string();
                    (
                        "service",
                        "other",
                        "used",
                        title,
                        description,
                        None,
                        None,
                        None,
                        None,
                        None,
                        Some(if global_idx.is_multiple_of(2) {
                            "online"
                        } else {
                            "local"
                        }),
                        Some(25.0 + (global_idx % 15) as f64),
                        Some(200.0 + (global_idx % 50) as f64),
                        Some(vec![
                            "Teaching License".to_string(),
                            "Math Degree".to_string(),
                        ]),
                        Some(10 + (global_idx % 5) as i32),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                }
                _ => {
                    let title = format!("Apartment for Rent {}", global_idx);
                    let description = "Modern apartment with strong local demand.".to_string();
                    (
                        "property",
                        "other",
                        "used",
                        title,
                        description,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        Some(if global_idx.is_multiple_of(2) {
                            "rent"
                        } else {
                            "sale"
                        }),
                        Some(if global_idx.is_multiple_of(3) {
                            "house"
                        } else {
                            "apartment"
                        }),
                        Some(80.0 + (global_idx % 120) as f64),
                        Some(1 + (global_idx % 4) as i32),
                        Some(1 + (global_idx % 3) as i32),
                        Some(2000 + (global_idx % 20) as i32),
                        Some(50.0 + (global_idx % 250) as f64),
                        Some("residential".to_string()),
                    )
                }
            };

            sqlx::query(
                "INSERT INTO listings (
                    listing_id, owner_id, schema_version, category, product_name, \"condition\",
                    price_currency, price_amount, country_code, country_name, city,
                    picture_urls, description, attributes, status, version, create_idempotency_key,
                    search_text, created_at, updated_at, sku, quantity, shipping_info, condition_details,
                    seller_notes, listing_type
                ) VALUES (
                    $1, $2, '1.0', $3, $4, $5,
                    'USD', $6, $7, $8, $9,
                    $10, $11, $12, 'active', 1, $13,
                    $14, now(), now(), $15, $16, $17, $18,
                    $19, $20
                )"
            )
            .bind(&listing_id)
            .bind(&owner_id)
            .bind(category)
            .bind(&title)
            .bind(condition)
            .bind(price)
            .bind(city_data.0)
            .bind(city_data.1)
            .bind(city_data.2)
            .bind(serde_json::json!([format!("https://example.com/{}.jpg", listing_id)]))
            .bind(description.clone())
            .bind(serde_json::json!({
                "brand": brands[(seller_idx + item_idx) % brands.len()],
                "model": format!("{}-{}", category, item_idx + 1),
                "seed": global_idx,
                "listing_type": listing_type,
            }))
            .bind(Uuid::new_v4().to_string())
            .bind(format!("{} {} {} {}", category, title, description, city_data.2))
            .bind(sku)
            .bind(quantity)
            .bind(shipping_info)
            .bind(condition_details)
            .bind(seller_notes)
            .bind(listing_type)
            .execute(&pool)
            .await?;

            match listing_type {
                "service" => {
                    sqlx::query(
                        "INSERT INTO service_listings (listing_id, service_type, hourly_rate, project_rate, qualifications, service_radius_km)
                         VALUES ($1, $2, $3, $4, $5, $6)"
                    )
                    .bind(&listing_id)
                    .bind(service_type.expect("service_type"))
                    .bind(hourly_rate)
                    .bind(project_rate)
                    .bind(serde_json::json!(qualifications.expect("qualifications")))
                    .bind(service_radius_km)
                    .execute(&pool)
                    .await?;
                }
                "property" => {
                    sqlx::query(
                        "INSERT INTO property_listings (listing_id, property_transaction_type, property_sub_type, area_sqm, bedrooms, bathrooms, year_built, lot_size_sqm, zoning)
                         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
                    )
                    .bind(&listing_id)
                    .bind(property_transaction_type.expect("property_transaction_type"))
                    .bind(property_sub_type.expect("property_sub_type"))
                    .bind(area_sqm)
                    .bind(bedrooms)
                    .bind(bathrooms)
                    .bind(year_built)
                    .bind(lot_size_sqm)
                    .bind(zoning)
                    .execute(&pool)
                    .await?;
                }
                _ => {}
            }

            listing_count += 1;
        }

        if (seller_idx + 1) % 100 == 0 {
            println!(
                "  Created listings for {}/{} sellers...",
                seller_idx + 1,
                num_sellers
            );
        }
    }
    println!("✅ Created {} listings", listing_count);

    println!(
        "\n⭐ Creating reviews (avg {} per listing)...",
        reviews_per_listing
    );
    let review_titles = [
        "Great product!",
        "Excellent seller",
        "Fast shipping",
        "Good quality",
        "Would buy again",
    ];
    let review_bodies = [
        "Really happy with this purchase.",
        "Seller was very professional.",
        "Item arrived quickly.",
    ];

    for seller_idx in 0..num_sellers {
        let owner_id = format!("seller_{}", seller_idx);
        let row = sqlx::query("SELECT seller_account_id FROM seller_accounts WHERE owner_id = $1")
            .bind(&owner_id)
            .fetch_one(&pool)
            .await?;
        let seller_account_id: String = row.get("seller_account_id");

        for item_idx in 0..num_listings_per_seller {
            let global_idx = seller_idx * num_listings_per_seller + item_idx + 1;
            let listing_id = format!("lst_{global_idx:07}");

            for review_idx in 0..reviews_per_listing {
                let review_id = Uuid::new_v4().to_string();
                let reviewer_id = format!("buyer_{}_{}", listing_id, review_idx);
                let rating: i32 = 3 + ((seller_idx + item_idx + review_idx) % 3) as i32;
                let title =
                    review_titles[(seller_idx + item_idx + review_idx) % review_titles.len()];
                let body =
                    review_bodies[(seller_idx + item_idx + review_idx) % review_bodies.len()];
                let status = if review_idx == 0 {
                    "approved"
                } else {
                    "pending"
                };

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
            println!(
                "  Created reviews for {}/{} sellers...",
                seller_idx + 1,
                num_sellers
            );
        }
    }
    println!(
        "✅ Created {} reviews",
        num_sellers * num_listings_per_seller * reviews_per_listing
    );

    println!("\n📊 Updating seller ratings...");
    let updated = sqlx::query(
        "UPDATE seller_accounts sa SET seller_rating = (
            SELECT ROUND(AVG(r.rating)::numeric, 2)::DECIMAL(3,2)
            FROM reviews r
            WHERE r.seller_account_id = sa.seller_account_id AND r.status = 'approved'
        ) WHERE EXISTS (
            SELECT 1 FROM reviews r2
            WHERE r2.seller_account_id = sa.seller_account_id AND r2.status = 'approved'
        )",
    )
    .execute(&pool)
    .await?;
    println!("✅ Updated ratings for {} sellers", updated.rows_affected());

    println!("\n🔢 Updating listings count...");
    let updated = sqlx::query(
        "UPDATE seller_accounts sa SET listings_created = (
            SELECT COUNT(*) FROM listings l WHERE l.owner_id = sa.owner_id
        )",
    )
    .execute(&pool)
    .await?;
    println!(
        "✅ Updated listings count for {} sellers",
        updated.rows_affected()
    );

    println!("\n🎉 Database population complete!");
    println!("📈 Summary:");
    println!("  - {} sellers", num_sellers);
    println!("  - {} listings", num_sellers * num_listings_per_seller);
    println!(
        "  - {} reviews",
        num_sellers * num_listings_per_seller * reviews_per_listing
    );
    println!("\n💡 Tip: Seed once, benchmark many against the populated database.");

    Ok(())
}
