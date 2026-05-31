pub mod csrf;
pub mod health;

use std::sync::Arc;

use axum::{
    Router,
    http::{HeaderName, HeaderValue, Method},
    routing::{get, post},
};
use sqlx::PgPool;
use tokio::sync::Semaphore;
use tower_http::cors::CorsLayer;

use crate::config::Config;
use crate::photos::platesolve::PlatesolveClient;

/// Maximum concurrent plate-solve uploads held in memory at once. Each
/// in-flight solve owns the full XISF body (capped by the
/// `/platesolve` route's `DefaultBodyLimit`) for the duration of the
/// upstream call + retries — so this bounds RSS. Sized for the Koyeb
/// Nano/Micro tier (≤512 MB); bump once we move to a larger instance.
const PLATESOLVE_MAX_CONCURRENT: usize = 1;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    pub storage: Arc<dyn crate::storage::Storage>,
    pub mailer: Arc<crate::mail::Mailer>,
    /// `None` when `APP_PLATESOLVE_BASE_URL` is unset — the
    /// `/api/photos/:id/platesolve` route is conditionally mounted
    /// only when this is `Some(_)`, so consumers will see a clean
    /// 404 rather than a runtime "not configured" check.
    pub platesolve: Option<Arc<PlatesolveClient>>,
    /// Bounds concurrent solves so we don't OOM on small Koyeb tiers.
    /// Always present even when `platesolve` is `None` so the handler
    /// can short-circuit before claiming a permit.
    pub platesolve_permits: Arc<Semaphore>,
}

/// Build a CORS layer that allows the given origin (e.g. the SvelteKit dev
/// server). Credentials are permitted so session cookies flow through.
/// Hard-coded to the dev origin for now; will be sourced from `Config` later.
pub fn cors_layer(allowed_origin: HeaderValue) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(allowed_origin)
        .allow_credentials(true)
        .allow_headers([HeaderName::from_static("content-type")])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
        ])
}

pub fn router(
    pool: PgPool,
    config: Config,
    storage: Arc<dyn crate::storage::Storage>,
    mailer: Arc<crate::mail::Mailer>,
    platesolve: Option<Arc<PlatesolveClient>>,
) -> Router {
    let state = AppState {
        pool,
        config: Arc::new(config),
        storage,
        mailer,
        platesolve,
        platesolve_permits: Arc::new(Semaphore::new(PLATESOLVE_MAX_CONCURRENT)),
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
            "/api/auth/verify-email",
            post(crate::auth::email_verify::verify),
        )
        .route(
            "/api/auth/resend-verification",
            post(crate::auth::email_verify::resend),
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
            "/api/photos/me/drafts",
            axum::routing::get(crate::photos::drafts_list::handler),
        )
        .route(
            "/api/photos/batch/apply",
            axum::routing::post(crate::photos::batch_apply::handler),
        )
        .route(
            "/api/photos/batch/publish",
            axum::routing::post(crate::photos::batch_publish::handler),
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
            "/api/photos/:id/apply-setup",
            axum::routing::post(crate::photos::apply_setup::apply),
        )
        .route(
            "/api/photos/:id/detach-setup",
            axum::routing::post(crate::photos::apply_setup::detach),
        )
        .route(
            "/api/photos/:id/targets",
            axum::routing::patch(crate::photos::targets::patch_targets),
        )
        .route(
            "/api/photos/:id/replace",
            post(crate::photos::replace::handler)
                .layer(axum::extract::DefaultBodyLimit::max(50 * 1024 * 1024)),
        )
        .route(
            "/api/photos/:id/platesolve-status",
            get(crate::photos::platesolve_status::handler),
        )
        .route(
            "/api/photos/:id/xisf-meta",
            get(crate::photos::xisf_display_handler::handler),
        )
        .route(
            "/api/photos/:id/processing",
            get(crate::photos::xisf_processing_handler::handler),
        )
        .route(
            "/api/photos/:id/celestial-objects",
            get(crate::celestial::handler::list),
        )
        .route(
            "/api/photos/:id/celestial-objects/recompute",
            post(crate::celestial::handler::recompute),
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
            "/api/users/by-handle/:handle/profile",
            axum::routing::get(crate::users::public_profile::get),
        )
        .route(
            "/api/users/by-handle/:handle/photos",
            axum::routing::get(crate::users::photos_feed::get),
        )
        .route(
            "/api/users/by-handle/:handle",
            axum::routing::get(crate::users::by_handle::handler),
        )
        .route(
            "/api/handles/redirect/:handle",
            axum::routing::get(crate::users::redirect_lookup::handler),
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
            "/api/me/cover",
            axum::routing::post(crate::users::cover::set),
        )
        .route(
            "/api/me/avatar/init",
            axum::routing::post(crate::users::avatar::init),
        )
        .route(
            "/api/me/avatar/finalize",
            axum::routing::post(crate::users::avatar::finalize),
        )
        .route(
            "/api/me/avatar",
            axum::routing::delete(crate::users::avatar::clear),
        )
        // Super-admin surface — each handler is guarded by the `AdminUser`
        // extractor (401 anonymous, 403 non-admin).
        .route(
            "/api/admin/settings",
            get(crate::admin::settings::get).put(crate::admin::settings::put),
        )
        .route("/api/admin/equipment", get(crate::admin::equipment::list))
        .route(
            "/api/admin/equipment/:id",
            axum::routing::patch(crate::admin::equipment::edit)
                .delete(crate::admin::equipment::delete),
        )
        .route(
            "/api/me/featured/order",
            axum::routing::patch(crate::users::featured::reorder),
        )
        .route(
            "/api/me/featured/:photo_id",
            axum::routing::post(crate::users::featured::pin).delete(crate::users::featured::unpin),
        )
        .route(
            "/api/me/profile",
            axum::routing::get(crate::users::profile::get)
                .put(crate::users::profile::put)
                .patch(crate::users::profile::put),
        )
        .route(
            "/api/me/preferences",
            axum::routing::get(crate::users::preferences::get).put(crate::users::preferences::put),
        )
        .route(
            "/api/me/storage",
            axum::routing::get(crate::users::storage::summary),
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
            "/api/equipment/catalog",
            axum::routing::get(crate::equipment::catalog_browse::handler),
        )
        .route(
            "/api/equipment/items",
            axum::routing::post(crate::equipment::items_create::handler),
        )
        .route(
            "/api/equipment/items/:id",
            axum::routing::get(crate::equipment::items_get::handler)
                .patch(crate::equipment::items_update::handler),
        )
        .route(
            "/api/equipment/setups",
            axum::routing::get(crate::equipment::setups::list::handler)
                .post(crate::equipment::setups::create::handler),
        )
        .route(
            "/api/equipment/setups/:id",
            axum::routing::get(crate::equipment::setups::get::handler)
                .patch(crate::equipment::setups::update::handler)
                .delete(crate::equipment::setups::delete::handler),
        )
        .route(
            "/api/uploads/init",
            axum::routing::post(crate::photos::upload_init::handler),
        )
        .route(
            "/api/uploads/:id",
            axum::routing::delete(crate::photos::upload_cancel::handler),
        )
        .route(
            "/api/uploads/:id/finalize",
            axum::routing::post(crate::photos::upload_finalize::handler),
        )
        .route(
            "/api/explore",
            axum::routing::get(crate::discovery::explore::get),
        )
        .route(
            "/api/targets",
            axum::routing::get(crate::discovery::target_index::list),
        )
        .route(
            "/api/photographers",
            axum::routing::get(crate::discovery::photographer_index::list),
        )
        .route(
            "/api/site/stats",
            axum::routing::get(crate::discovery::site_stats::get),
        )
        .route(
            "/api/targets/:slug",
            axum::routing::get(crate::discovery::target::get),
        )
        .route(
            "/api/tags/:slug",
            axum::routing::get(crate::discovery::tag::get),
        )
        .route(
            "/api/equipment/:kind/:slug",
            axum::routing::get(crate::discovery::equipment::get),
        )
        .route(
            "/api/categories/:cat",
            axum::routing::get(crate::discovery::category::get),
        )
        .route(
            "/api/search",
            axum::routing::get(crate::discovery::search::get),
        );

    // Mount the side-channel plate-solve endpoint only when the
    // upstream client is configured. Tests / dev installs without
    // `APP_PLATESOLVE_BASE_URL` get a clean 404 from axum's matcher
    // rather than a 500 from a runtime "not configured" check.
    if state.platesolve.is_some() {
        router = router.route(
            "/api/photos/:id/platesolve",
            post(crate::photos::platesolve_upload::handler).layer(
                axum::extract::DefaultBodyLimit::max(crate::photos::platesolve::MAX_XISF_BYTES),
            ),
        );
    }

    // Mount the dev CDN only when CDN_BASE_URL points back at this process.
    // In production, CloudFront is in front and this route is not needed.
    let mount_cdn_dev = state.config.cdn_local_fallback
        || state.config.cdn_base_url.contains("localhost")
        || state.config.cdn_base_url.contains("127.0.0.1");
    if mount_cdn_dev {
        router = router.route(
            "/cdn/img/:id",
            axum::routing::get(crate::storage::cdn_dev::handler),
        );
    }

    router.with_state(state)
}
