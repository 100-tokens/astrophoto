//! Demo seed: creates `demo@astrophoto.example` (password `demoaccount`)
//! and uploads any JPEG/PNG files in `backend/seeds/fixtures/` via the
//! same pipeline as the HTTP upload handler. Idempotent.
//!
//! Run: `just seed`

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use astrophoto::auth::password;
use astrophoto::photos::pipeline;
use astrophoto::storage::S3Storage;
use astrophoto::users::queries as user_q;
use astrophoto::{Config, db};
use bytes::Bytes;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cfg = Config::from_env();

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

    let demo_email = "demo@astrophoto.example";
    let demo_user = match user_q::find_by_email(&pool, demo_email).await? {
        Some(u) => {
            tracing::info!(user_id = %u.id, "demo user already exists");
            u
        }
        None => {
            let hash = password::hash("demoaccount".into()).await?;
            let u = user_q::create_with_password(
                &pool,
                demo_email,
                "Demo Astrographer",
                &hash,
            )
            .await?;
            tracing::info!(user_id = %u.id, "demo user created");
            u
        }
    };

    let fixtures_dir = PathBuf::from("seeds/fixtures");
    let mut entries: Vec<_> = match std::fs::read_dir(&fixtures_dir) {
        Ok(it) => it.filter_map(Result::ok).collect(),
        Err(_) => {
            tracing::warn!(dir = %fixtures_dir.display(), "no fixtures dir; nothing to seed");
            return Ok(());
        }
    };
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) if n.ends_with(".jpg") || n.ends_with(".jpeg") || n.ends_with(".png") => {
                n.to_string()
            }
            _ => continue,
        };

        if photo_already_imported(&pool, demo_user.id, &name).await? {
            tracing::info!(file = %name, "already imported, skipping");
            continue;
        }

        let bytes = Bytes::from(
            std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?,
        );
        let mime = if name.ends_with(".png") {
            "image/png"
        } else {
            "image/jpeg"
        };
        let id = pipeline::process(
            &pool,
            storage.clone(),
            demo_user.id,
            &name,
            mime,
            None,
            None,
            bytes,
        )
        .await?;
        tracing::info!(file = %name, photo_id = %id, "imported");
    }

    Ok(())
}

async fn photo_already_imported(
    pool: &sqlx::PgPool,
    owner_id: uuid::Uuid,
    name: &str,
) -> Result<bool, astrophoto::AppError> {
    let row = sqlx::query!(
        "select 1 as one from photos where owner_id = $1 and original_name = $2 limit 1",
        owner_id,
        name
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}
