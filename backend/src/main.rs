#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use astrophoto::{Config, db, http, photos::platesolve::PlatesolveClient, storage::S3Storage};
use axum::http::{HeaderValue, header};
use tokio::net::TcpListener;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env from the workspace root for dev convenience. In prod / staging
    // (Koyeb, CI), env vars are injected by the runtime and dotenvy is a no-op.
    let _ = dotenvy::from_path("../.env");
    let _ = dotenvy::dotenv();

    let cfg = Config::from_env();
    init_tracing(&cfg.log);

    let pool = db::connect(&cfg.database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Fill the opposition_doy cache for any target seeded before the column
    // existed (prod is seeded; seeds are manual). Idempotent and cheap once
    // populated — a no-op SELECT on subsequent boots.
    match astrophoto::discovery::opposition::backfill_missing(&pool).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(filled = n, "backfilled targets.opposition_doy"),
        Err(e) => tracing::warn!(error = %e, "opposition_doy backfill failed; continuing"),
    }

    let storage = Arc::new(
        S3Storage::new(
            cfg.s3_endpoint.as_deref(),
            &cfg.s3_region,
            &cfg.s3_bucket,
            &cfg.s3_access_key,
            &cfg.s3_secret_key,
            cfg.s3_path_style,
        )
        .await?,
    );

    let mailer = std::sync::Arc::new(astrophoto::mail::Mailer::from_env(&cfg)?);

    // Build the plate-solve client up front so config errors surface
    // at boot, not at first solve attempt. `from_config` returns
    // `Ok(None)` when the feature is unset, `Err(_)` if the URL is
    // set but the API key is missing/empty.
    let platesolve = PlatesolveClient::from_config(&cfg)?.map(Arc::new);

    // Spawn background workers before handing pool/storage to the router.
    astrophoto::jobs::purge_deletions::spawn(pool.clone(), storage.clone());
    astrophoto::photos::cleanup::spawn_periodic(pool.clone(), storage.clone());

    // Fail fast: prod/staging (session_secure=true) must name the real
    // frontend origin. The localhost fallback below is dev-only — with it,
    // the service boots green while the CSRF origin guard 403s every
    // cookie-authenticated mutation, which is far harder to diagnose than
    // a boot panic naming the missing variable.
    if cfg.session_secure && cfg.cors_origin.is_none() {
        panic!(
            "APP_CORS_ORIGIN must be set when APP_SESSION_SECURE=true (prod/staging); \
             the http://localhost:5173 default is dev-only"
        );
    }

    // Allow the SvelteKit app to reach the backend with credentials.
    // Reads APP_CORS_ORIGIN from the environment; falls back to the dev server.
    let cors_origin_str = cfg
        .cors_origin
        .as_deref()
        .unwrap_or("http://localhost:5173")
        .trim_end_matches('/')
        .to_string();
    let cors_origin: HeaderValue = cors_origin_str
        .parse()
        .expect("APP_CORS_ORIGIN is not a valid HTTP origin header value");

    // CSRF Origin allowlist for cookie-authenticated mutations. The frontend
    // CORS origin is always allowed; APP_EXTRA_BROWSER_ORIGINS (comma-separated)
    // covers any additional browser-reachable frontend host (e.g. a raw Koyeb
    // *-web-* host alongside the canonical www) so users there are not 403'd.
    let mut allowed = std::collections::HashSet::from([cors_origin_str]);
    if let Ok(extra) = std::env::var("APP_EXTRA_BROWSER_ORIGINS") {
        for o in extra.split(',') {
            let o = o.trim().trim_end_matches('/');
            if !o.is_empty() {
                allowed.insert(o.to_string());
            }
        }
    }
    let allowed_origins = http::csrf::AllowedOrigins(allowed);

    // Layer order (tower applies them router-first = innermost-first): the
    // origin guard sits innermost (closest to the router), CORS wraps it so an
    // OPTIONS preflight is answered before the guard runs, TraceLayer outermost.
    // Security headers: the backend origin is directly browser-reachable
    // (OAuth callback sets the session cookie there; /cdn/img serves image
    // bytes under APP_CDN_LOCAL_FALLBACK), so it needs its own hardening —
    // the frontend's hooks.server.ts headers never cover this host.
    let mut app = http::router(pool, cfg.clone(), storage, mailer, platesolve)
        .layer(axum::middleware::from_fn_with_state(
            allowed_origins,
            http::csrf::origin_guard,
        ))
        .layer(http::cors_layer(cors_origin))
        .layer(TraceLayer::new_for_http())
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ));
    // HSTS only where HTTPS is guaranteed (prod/staging). Sending it from
    // plain-HTTP dev would poison localhost for every other local project.
    if cfg.session_secure {
        app = app.layer(SetResponseHeaderLayer::if_not_present(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        ));
    }

    let listener = TcpListener::bind(&cfg.bind).await?;
    tracing::info!(bind = %cfg.bind, "astrophoto listening");
    // Graceful shutdown: on SIGTERM (Koyeb deploy/stop) or ctrl-c, stop
    // accepting new connections and drain in-flight requests (finalize,
    // replace) instead of aborting them mid-write. Rows orphaned by a hard
    // kill are still recovered by `photos::cleanup::sweep_stuck_pipeline`.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

/// Resolves when the process receives SIGTERM or SIGINT/ctrl-c.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install ctrl-c handler");
    };
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received; draining in-flight requests");
}

fn init_tracing(log: &str) {
    use std::io::IsTerminal;
    let filter = EnvFilter::try_new(log).unwrap_or_else(|_| EnvFilter::new("info"));
    let layer = if std::io::stdout().is_terminal() {
        fmt::layer().compact().boxed()
    } else {
        fmt::layer().json().boxed()
    };
    tracing_subscriber::registry()
        .with(filter)
        .with(layer)
        .init();
}
