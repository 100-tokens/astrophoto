//! One-shot: parse + store `processing_json` for XISF photos that
//! predate the processing-history feature. Reads only the header via
//! `Storage::get_range` (a two-step exact read: the 16-byte length
//! prefix, then `16 + header_len`), so memory is bounded to the real
//! header size — not the 50–500 MB master.
//!
//! Default: dry-run (prints counts only). Pass `--apply` to write.
//! Idempotent — the eligibility filter excludes photos that already
//! have a `processing_json`, so a second `--apply` is a no-op.
//!
//! Side-channel-only uploads (XISF not stored in S3) are skipped.

use anyhow::Result;
use bytes::Bytes;
use clap::Parser;

use astrophoto::storage::Storage;

/// Hard ceiling on header size; a larger length-prefix means a corrupt
/// or hostile file — skip with a warning rather than allocate.
const MAX_HEADER_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Parser, Debug)]
#[command(about = "Backfill photos.processing_json from stored XISF headers.")]
struct Args {
    /// Without this flag, runs in dry-run mode and only prints counts.
    #[arg(long, default_value_t = false)]
    apply: bool,
}

#[derive(Default, Debug, PartialEq)]
pub struct BackfillCounts {
    pub eligible: usize,
    pub parsed: usize,
    pub no_history: usize,
    pub missing_object: usize,
    pub errors: usize,
}

/// Fetch exactly the XISF envelope + header (`0..16 + header_len`).
/// `Ok(None)` when the object is absent (side-channel upload) or not an
/// XISF. `Err` only on an implausible length prefix.
pub async fn fetch_header(storage: &dyn Storage, key: &str) -> Result<Option<Bytes>> {
    let Some(prefix) = storage.get_range(key, 0, 15).await? else {
        return Ok(None);
    };
    if prefix.len() < 16 || &prefix[0..8] != b"XISF0100" {
        return Ok(None);
    }
    let hlen = u32::from_le_bytes([prefix[8], prefix[9], prefix[10], prefix[11]]) as u64;
    if hlen == 0 || hlen > MAX_HEADER_BYTES {
        anyhow::bail!("implausible XISF header length {hlen} for {key}");
    }
    Ok(storage.get_range(key, 0, 16 + hlen - 1).await?)
}

pub async fn run_once(
    pool: &sqlx::PgPool,
    storage: &dyn Storage,
    apply: bool,
) -> Result<BackfillCounts> {
    let mut counts = BackfillCounts::default();

    // Runtime query (not the `query!` macro) so this compiles offline
    // before `processing_json` lands in the .sqlx cache.
    let rows: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        "select id, storage_key from photos \
         where mime = 'application/x-xisf' and processing_json is null",
    )
    .fetch_all(pool)
    .await?;
    counts.eligible = rows.len();

    for (id, key) in rows {
        let bytes = match fetch_header(storage, &key).await {
            Ok(Some(b)) => b,
            Ok(None) => {
                counts.missing_object += 1;
                continue;
            }
            Err(e) => {
                tracing::warn!(%id, error = %e, "backfill: header read failed");
                counts.errors += 1;
                continue;
            }
        };
        match astrophoto::photos::xisf_processing::parse_xisf(&bytes) {
            Ok(Some(report)) => {
                counts.parsed += 1;
                if apply {
                    let json = serde_json::to_value(&report)?;
                    sqlx::query("update photos set processing_json = $1 where id = $2")
                        .bind(json)
                        .bind(id)
                        .execute(pool)
                        .await?;
                }
            }
            Ok(None) => counts.no_history += 1,
            Err(e) => {
                tracing::warn!(%id, error = %e, "backfill: parse failed");
                counts.errors += 1;
            }
        }
    }
    Ok(counts)
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::from_path("../.env");
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let cfg = astrophoto::Config::from_env();
    let pool = astrophoto::db::connect(&cfg.database_url).await?;
    let storage = astrophoto::storage::S3Storage::new(
        cfg.s3_endpoint.as_deref(),
        &cfg.s3_region,
        &cfg.s3_bucket,
        &cfg.s3_access_key,
        &cfg.s3_secret_key,
        cfg.s3_path_style,
    )
    .await?;

    let counts = run_once(&pool, &storage, args.apply).await?;
    tracing::info!(
        eligible = counts.eligible,
        parsed = counts.parsed,
        no_history = counts.no_history,
        missing_object = counts.missing_object,
        errors = counts.errors,
        apply = args.apply,
        "backfill-processing complete"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use astrophoto::storage::MemoryStorage;

    /// Build a valid XISF binary whose header embeds `inner_history` as
    /// escaped element text — the same shape PixInsight writes.
    fn synthetic_xisf(inner_history: &str) -> Vec<u8> {
        let escaped = inner_history
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        let header = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<xisf version="1.0" xmlns="http://www.pixinsight.com/xisf">
<Image geometry="10:10:1" sampleFormat="Float32" colorSpace="Gray">
<Property id="XISF:CreatorApplication" type="String" value="PixInsight 1.9.2"/>
<Property id="PixInsight:ProcessingHistory" type="String">{escaped}</Property>
</Image>
</xisf>"#
        );
        let mut buf = Vec::new();
        buf.extend_from_slice(b"XISF0100");
        buf.extend_from_slice(&(header.len() as u32).to_le_bytes());
        buf.extend_from_slice(&[0u8; 4]);
        buf.extend_from_slice(header.as_bytes());
        buf
    }

    const HISTORY: &str = r#"<?xml version="1.0"?><ProcessingHistory version="1.0">
        <instance class="ChannelCombination" version="256" enabled="true">
            <time start="2025-07-05T12:10:43.438Z" span="0.99"/>
            <parameter id="colorSpace" value="RGB"/>
        </instance>
        <instance class="CurvesTransformation" version="256" enabled="true">
            <time start="2025-07-05T12:23:35.874Z" span="0.04"/>
            <table id="K" rows="2"><tr><td id="x" value="0.0"/><td id="y" value="0.0"/></tr>
            <tr><td id="x" value="1.0"/><td id="y" value="1.0"/></tr></table>
        </instance></ProcessingHistory>"#;

    // ── fetch_header: no DB needed, runs without Docker ──────────────

    #[tokio::test]
    async fn fetch_header_two_step_reads_exact_header() {
        let s = MemoryStorage::new();
        let blob = synthetic_xisf(HISTORY);
        let header_len = blob.len(); // tiny file: header is essentially the whole thing
        s.put("k", "application/x-xisf", Bytes::from(blob.clone()))
            .await
            .unwrap();
        let got = fetch_header(&s, "k").await.unwrap().unwrap();
        // We fetched 16 + header_len bytes (the whole synthetic file here).
        assert_eq!(got.len(), header_len);
        assert_eq!(&got[0..8], b"XISF0100");
        // And it parses end-to-end.
        let report = astrophoto::photos::xisf_processing::parse_xisf(&got)
            .unwrap()
            .unwrap();
        assert_eq!(report.pipeline.len(), 2);
    }

    #[tokio::test]
    async fn fetch_header_absent_object_is_none() {
        let s = MemoryStorage::new();
        assert!(fetch_header(&s, "missing").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn fetch_header_non_xisf_is_none() {
        let s = MemoryStorage::new();
        s.put("k", "application/octet-stream", Bytes::from_static(b"not an xisf file...."))
            .await
            .unwrap();
        assert!(fetch_header(&s, "k").await.unwrap().is_none());
    }

    // ── run_once: needs a real DB (testcontainers / Docker) ──────────

    #[tokio::test]
    async fn apply_populates_processing_json() {
        use sqlx::postgres::PgPoolOptions;
        use testcontainers::ImageExt;
        use testcontainers::runners::AsyncRunner;
        use testcontainers_modules::postgres::Postgres as PgImage;
        use uuid::Uuid;

        let pg = PgImage::default()
            .with_tag("16-alpine")
            .start()
            .await
            .unwrap();
        let host = pg.get_host().await.unwrap();
        let port = pg.get_host_port_ipv4(5432).await.unwrap();
        let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        // Seed a user + an XISF photo whose storage object exists.
        let suffix = Uuid::new_v4().to_string().replace('-', "");
        let short = &suffix[..8];
        // Runtime queries (not the `query!` macro) so the test compiles
        // offline before `processing_json` is in the .sqlx cache.
        let user_id: Uuid = sqlx::query_scalar(
            "insert into users (email, display_name, handle) values ($1, 'T', $2) returning id",
        )
        .bind(format!("u-{short}@test.local"))
        .bind(format!("u-{short}"))
        .fetch_one(&pool)
        .await
        .unwrap();
        let photo_id = Uuid::new_v4();
        sqlx::query(
            "insert into photos (id, owner_id, storage_key, original_name, bytes, mime, status, short_id, original_uploaded_at) \
             values ($1, $2, 'originals/x.xisf', 'm20.xisf', 100, 'application/x-xisf', 'ready', $3, now())",
        )
        .bind(photo_id)
        .bind(user_id)
        .bind(short.to_uppercase())
        .execute(&pool)
        .await
        .unwrap();

        let storage = MemoryStorage::new();
        storage
            .put(
                "originals/x.xisf",
                "application/x-xisf",
                Bytes::from(synthetic_xisf(HISTORY)),
            )
            .await
            .unwrap();

        // Dry-run writes nothing.
        let dry = run_once(&pool, &storage, false).await.unwrap();
        assert_eq!(dry.eligible, 1);
        assert_eq!(dry.parsed, 1);
        let still_null: Option<serde_json::Value> =
            sqlx::query_scalar("select processing_json from photos where id = $1")
                .bind(photo_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(still_null.is_none(), "dry-run must not write");

        // Apply populates the column with the parsed pipeline.
        let applied = run_once(&pool, &storage, true).await.unwrap();
        assert_eq!(applied.parsed, 1);
        let json: Option<serde_json::Value> =
            sqlx::query_scalar("select processing_json from photos where id = $1")
                .bind(photo_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        let json = json.expect("processing_json populated");
        assert_eq!(json["pipeline"].as_array().unwrap().len(), 2);
        assert_eq!(json["creatorApp"], "PixInsight 1.9.2");

        // Second apply is a no-op (eligibility excludes non-null rows).
        let again = run_once(&pool, &storage, true).await.unwrap();
        assert_eq!(again.eligible, 0);
    }
}
