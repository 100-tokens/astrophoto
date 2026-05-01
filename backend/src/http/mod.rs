pub mod health;

use axum::{Router, routing::get};
use sqlx::PgPool;

pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/healthz", get(health::healthz))
        .with_state(pool)
}
