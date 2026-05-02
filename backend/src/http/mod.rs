pub mod health;

use std::sync::Arc;

use axum::{
    Router,
    http::{HeaderName, HeaderValue, Method},
    routing::{get, post},
};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    pub storage: Arc<dyn crate::storage::Storage>,
}

/// Build a CORS layer that allows the given origin (e.g. the SvelteKit dev
/// server). Credentials are permitted so session cookies flow through.
/// Hard-coded to the dev origin for now; will be sourced from `Config` later.
pub fn cors_layer(allowed_origin: HeaderValue) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(allowed_origin)
        .allow_credentials(true)
        .allow_headers([HeaderName::from_static("content-type")])
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
}

pub fn router(pool: PgPool, config: Config, storage: Arc<dyn crate::storage::Storage>) -> Router {
    let state = AppState {
        pool,
        config: Arc::new(config),
        storage,
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
        .route(
            "/api/photos",
            post(crate::photos::upload::handler)
                .get(crate::photos::list::handler)
                .layer(axum::extract::DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route("/api/photos/:id", get(crate::photos::get::handler))
        .route(
            "/api/photos/:id/thumb/:size",
            get(crate::photos::serve::thumb),
        )
        .with_state(state)
}
