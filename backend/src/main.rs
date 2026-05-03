#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use astrophoto::{Config, db, http, storage::S3Storage};
use axum::http::HeaderValue;
use tokio::net::TcpListener;
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

    // Spawn background workers before handing pool/storage to the router.
    astrophoto::jobs::purge_deletions::spawn(pool.clone(), storage.clone());
    astrophoto::jobs::orphan_reaper::spawn(pool.clone(), storage.clone());

    // Allow the SvelteKit app to reach the backend with credentials.
    // Reads APP_CORS_ORIGIN from the environment; falls back to the dev server.
    let cors_origin: HeaderValue = cfg
        .cors_origin
        .as_deref()
        .unwrap_or("http://localhost:5173")
        .parse()
        .expect("APP_CORS_ORIGIN is not a valid HTTP origin header value");
    let app = http::router(pool, cfg.clone(), storage, mailer)
        .layer(http::cors_layer(cors_origin))
        .layer(TraceLayer::new_for_http());

    let listener = TcpListener::bind(&cfg.bind).await?;
    tracing::info!(bind = %cfg.bind, "astrophoto listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
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
