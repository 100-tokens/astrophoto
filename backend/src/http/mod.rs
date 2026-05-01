pub mod health;

use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
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
        .route("/api/auth/signup", post(crate::auth::signup::handler))
        .route("/api/auth/login", post(crate::auth::login::handler))
        .route("/api/auth/me", get(crate::auth::me::handler))
        .route("/api/auth/logout", post(crate::auth::logout::handler))
        .route(
            "/api/auth/oauth/google/start",
            get(crate::auth::oauth_google::start),
        )
        .route(
            "/api/auth/oauth/google/callback",
            get(crate::auth::oauth_google::callback),
        )
        .with_state(state)
}
