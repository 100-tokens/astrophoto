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
    pub mailer: Arc<crate::mail::Mailer>,
}

/// Build a CORS layer that allows the given origin (e.g. the SvelteKit dev
/// server). Credentials are permitted so session cookies flow through.
/// Hard-coded to the dev origin for now; will be sourced from `Config` later.
pub fn cors_layer(allowed_origin: HeaderValue) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(allowed_origin)
        .allow_credentials(true)
        .allow_headers([HeaderName::from_static("content-type")])
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
}

pub fn router(
    pool: PgPool,
    config: Config,
    storage: Arc<dyn crate::storage::Storage>,
    mailer: Arc<crate::mail::Mailer>,
) -> Router {
    let state = AppState {
        pool,
        config: Arc::new(config),
        storage,
        mailer,
    };
    let mut router = Router::new()
        .route("/healthz", get(health::healthz))
        .route(
            "/api/auth/handle-check",
            get(crate::auth::handle_check::handler),
        )
        .route("/api/auth/signup", post(crate::auth::signup::handler))
        .route("/api/auth/login", post(crate::auth::login::handler))
        .route("/api/auth/me", get(crate::auth::me::handler))
        .route("/api/auth/logout", post(crate::auth::logout::handler))
        .route(
            "/api/auth/password-reset/request",
            post(crate::auth::password_reset::request),
        )
        .route(
            "/api/auth/password-reset/confirm",
            post(crate::auth::password_reset::confirm),
        )
        .route(
            "/api/me/password-change",
            post(crate::auth::password_change::change),
        )
        .route(
            "/api/me/email-change/request",
            post(crate::auth::email_change::request),
        )
        .route(
            "/api/auth/email-change/confirm",
            post(crate::auth::email_change::confirm),
        )
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
            axum::routing::get(crate::photos::list::handler),
        )
        .route(
            "/api/photos/:id",
            get(crate::photos::get::handler)
                .put(crate::photos::metadata::handler)
                .delete(crate::photos::delete::handler),
        )
        .route(
            "/api/photos/:id/publish",
            post(crate::photos::publish::handler),
        )
        .route(
            "/api/photos/:id/replace",
            post(crate::photos::replace::handler)
                .layer(axum::extract::DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route(
            "/api/photos/:id/thumb/:size",
            get(crate::photos::serve::thumb),
        )
        .route(
            "/api/users/:id",
            axum::routing::get(crate::users::get::handler),
        )
        .route(
            "/api/photos/:id/appreciate",
            axum::routing::post(crate::engagement::appreciations::appreciate)
                .delete(crate::engagement::appreciations::unappreciate),
        )
        .route(
            "/api/photos/:id/appreciations/count",
            axum::routing::get(crate::engagement::appreciations::count),
        )
        .route(
            "/api/photos/:id/appreciation-state",
            axum::routing::get(crate::engagement::appreciations::state_for_user),
        )
        .route(
            "/api/photos/:id/comments",
            axum::routing::get(crate::engagement::comments::list)
                .post(crate::engagement::comments::create),
        )
        .route(
            "/api/comments/:id",
            axum::routing::delete(crate::engagement::comments::delete),
        )
        .route(
            "/api/users/:id/follow",
            axum::routing::post(crate::engagement::follows::follow)
                .delete(crate::engagement::follows::unfollow),
        )
        .route(
            "/api/users/:id/followers/count",
            axum::routing::get(crate::engagement::follows::followers_count),
        )
        .route(
            "/api/users/:id/following/count",
            axum::routing::get(crate::engagement::follows::following_count),
        )
        .route(
            "/api/me/profile",
            axum::routing::get(crate::users::profile::get).put(crate::users::profile::put),
        )
        .route(
            "/api/me/preferences",
            axum::routing::get(crate::users::preferences::get).put(crate::users::preferences::put),
        )
        .route(
            "/api/me/sessions",
            axum::routing::get(crate::users::sessions::list),
        )
        .route(
            "/api/me/sessions/:id",
            axum::routing::delete(crate::users::sessions::revoke),
        )
        .route(
            "/api/me/sessions/sign-out-others",
            axum::routing::post(crate::users::sessions::sign_out_others),
        )
        .route(
            "/api/me/delete-request",
            post(crate::users::deletion::request),
        )
        .route(
            "/api/me/delete-cancel",
            post(crate::users::deletion::cancel),
        )
        .route(
            "/api/photos/by-permalink/:handle/:short_id",
            axum::routing::get(crate::photos::permalink::lookup),
        )
        .route(
            "/api/photos/by-uuid/:id",
            axum::routing::get(crate::photos::redirect::redirect_uuid_to_canonical),
        )
        .route("/api/me/photos/count", get(crate::photos::count::handler))
        .route("/api/me/stats", get(crate::users::stats::handler))
        .route("/api/me/export.json", get(crate::users::export::handler))
        .route(
            "/api/me/handle",
            axum::routing::post(crate::users::handle::rename),
        )
        .route(
            "/api/tags/autocomplete",
            axum::routing::get(crate::photos::tags_autocomplete::handler),
        )
        .route(
            "/api/targets/autocomplete",
            axum::routing::get(crate::photos::targets_autocomplete::handler),
        )
        .route(
            "/api/equipment/autocomplete",
            axum::routing::get(crate::equipment::autocomplete::handler),
        )
        .route(
            "/api/uploads/init",
            axum::routing::post(crate::photos::upload_init::handler),
        )
        .route(
            "/api/uploads/:id/finalize",
            axum::routing::post(crate::photos::upload_finalize::handler),
        );

    // Mount the dev CDN only when CDN_BASE_URL points back at this process.
    // In production, CloudFront is in front and this route is not needed.
    if state.config.cdn_base_url.contains("localhost")
        || state.config.cdn_base_url.contains("127.0.0.1")
    {
        router = router.route(
            "/cdn/img/:id",
            axum::routing::get(crate::storage::cdn_dev::handler),
        );
    }

    router.with_state(state)
}
