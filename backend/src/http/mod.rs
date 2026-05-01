pub mod health;

use std::sync::Arc;

use axum::{Router, routing::get};
use sqlx::PgPool;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
}

pub fn router(pool: PgPool, config: Config) -> Router {
    let state = AppState {
        pool,
        config: Arc::new(config),
    };
    Router::new()
        .route("/healthz", get(health::healthz))
        .route(
            "/api/auth/signup",
            axum::routing::post(crate::auth::signup::handler),
        )
        .with_state(state)
}
