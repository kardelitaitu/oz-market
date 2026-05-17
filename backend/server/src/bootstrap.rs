use sqlx::{migrate::Migrator, PgPool};

pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub async fn apply_schema(pool: &PgPool) -> Result<(), sqlx::Error> {
    MIGRATOR
        .run(pool)
        .await
        .map_err(|err| sqlx::Error::Migrate(Box::new(err)))
}
