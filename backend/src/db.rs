use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::AppError;

pub async fn connect(database_url: &str) -> Result<PgPool, AppError> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(database_url)
        .await?;
    Ok(pool)
}
