#![allow(clippy::unwrap_used, clippy::expect_used)]

use anyhow::Result;
use astrophoto::{Config, db, http};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = Config::from_env();
    init_tracing(&cfg.log);

    let pool = db::connect(&cfg.database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let app = http::router(pool, cfg.clone()).layer(TraceLayer::new_for_http());

    let listener = TcpListener::bind(&cfg.bind).await?;
    tracing::info!(bind = %cfg.bind, "astrophoto listening");
    axum::serve(listener, app).await?;
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
