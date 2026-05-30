//! Integration test: seed_pgc against a fresh testcontainer Postgres.
//! Verifies UPSERT, dedup with NGC, and idempotence.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::process::Command;
use testcontainers::ImageExt;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;

/// Spin up a fresh Postgres testcontainer, run migrations, and return
/// `(pool, url)`. The container is leaked (`mem::forget`) so it survives
/// for the test process — mirrors `tests/common/mod.rs::make_app_and_pool`.
async fn fresh_db() -> (sqlx::PgPool, String) {
    let pg = PgImage::default()
        .with_tag("16-alpine")
        .start()
        .await
        .unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = astrophoto::db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    std::mem::forget(pg);
    (pool, url)
}

#[tokio::test]
async fn seed_pgc_inserts_and_dedupes_and_is_idempotent() {
    let (pool, url) = fresh_db().await;

    // Seed an NGC row that the PGC fixture (NGC0224 ref) should dedup against.
    sqlx::query(
        "insert into targets (slug, canonical_name, kind, right_ascension, declination)
         values ('ngc-224', 'Andromeda Galaxy', 'ngc', 10.6847083, 41.2691055)",
    )
    .execute(&pool)
    .await
    .unwrap();

    // 3-row fixture: NGC0224 (skipped by dedup), one named PGC, one anonymous PGC.
    let csv = "\
pgc,objname,ra2000,de2000,bt,logd25,logr25,pa
2557,NGC0224,10.6847083,41.2691055,4.36,2.337,0.502,35.0
12345,Some Galaxy,123.45,-12.34,17.0,0.6,0.0,
99999,,234.5,5.0,18.5,0.5,,
";
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), csv).unwrap();

    // First run.
    let status = Command::new(env!("CARGO_BIN_EXE_seed_pgc"))
        .env("DATABASE_URL", &url)
        .env("PGC_DATA_PATH", tmp.path())
        .status()
        .unwrap();
    assert!(status.success(), "seed_pgc binary failed on first run");

    // Dedup dropped NGC0224 → only 2 PGC rows.
    let row_count: i64 = sqlx::query_scalar("select count(*) from targets where kind = 'pgc'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row_count, 2, "expected dedup to drop NGC0224 → 2 PGC rows");

    // NGC0224's canonical_name was preserved (PGC must never overwrite NGC text).
    let ngc_canonical: String =
        sqlx::query_scalar("select canonical_name from targets where slug = 'ngc-224'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(ngc_canonical, "Andromeda Galaxy");

    // Anonymous PGC got a synthetic canonical_name.
    let anon_canonical: String =
        sqlx::query_scalar("select canonical_name from targets where slug = 'pgc-99999'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(anon_canonical, "PGC 99999");

    // Re-run → idempotent: same row count, no new inserts.
    let status2 = Command::new(env!("CARGO_BIN_EXE_seed_pgc"))
        .env("DATABASE_URL", &url)
        .env("PGC_DATA_PATH", tmp.path())
        .status()
        .unwrap();
    assert!(status2.success(), "seed_pgc binary failed on second run");
    let row_count2: i64 = sqlx::query_scalar("select count(*) from targets where kind = 'pgc'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row_count2, 2, "second run must be idempotent");
}
