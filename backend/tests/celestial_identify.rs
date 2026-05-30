//! Integration test for `celestial::identify`. Confirms the cone search
//! and write-time filter write the expected `photo_targets` rows with
//! `source='plate_solve'`. Per project convention (Docker often
//! constrained locally), this test is exercised on staging CI.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use astrophoto::celestial::identify;
use testcontainers::ImageExt;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;
use uuid::Uuid;

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
async fn identify_writes_expected_rows_with_filter() {
    let (pool, _url) = fresh_db().await;

    // Seed 3 targets around RA=180, Dec=0 — the test field center.
    sqlx::query(
        r#"insert into targets (slug, canonical_name, kind, right_ascension,
                                declination, magnitude_v, object_type, major_axis_arcmin)
           values
             ('m99-test', 'Test M', 'messier', 180.000, 0.000, 5.0, 'G', 20.0),
             ('ngc-test', 'Test NGC', 'ngc',   180.100, 0.050, 9.0, 'G', 3.0),
             ('pgc-far',  'Test PGC far', 'pgc', 181.000, 1.500, 17.5, 'G', 0.2)"#,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Fake photo solved at 180/0 with a 0.4° FOV (1440 px × 1″/px).
    let owner = Uuid::new_v4();
    sqlx::query("insert into users (id, display_name) values ($1, 'tester')")
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();
    let photo_id = Uuid::new_v4();
    sqlx::query(
        r#"insert into photos (id, owner_id, storage_key, original_name, mime,
                               ra_deg, dec_deg, platesolve_pixel_scale_arcsec,
                               width, height)
           values ($1, $2, 'k', 'n', 'image/jpeg', 180.0, 0.0, 1.0, 1440, 1440)"#,
    )
    .bind(photo_id)
    .bind(owner)
    .execute(&pool)
    .await
    .unwrap();

    // Run identify inside a transaction (mirrors how save_result calls it).
    let mut tx = pool.begin().await.unwrap();
    let outcome = identify(photo_id, &mut tx).await.unwrap();
    tx.commit().await.unwrap();

    // M99 + NGC clearly in the field; PGC at (181, 1.5) is outside the
    // ~0.3° cone radius around (180, 0).
    let kept_slugs: Vec<String> = sqlx::query_scalar(
        r#"select t.slug from photo_targets pt
             join targets t on t.id = pt.target_id
            where pt.photo_id = $1 and pt.source = 'plate_solve'
            order by t.slug"#,
    )
    .bind(photo_id)
    .fetch_all(&pool)
    .await
    .unwrap();

    assert!(
        kept_slugs.contains(&"m99-test".to_string()),
        "kept_slugs = {:?}",
        kept_slugs
    );
    assert!(
        kept_slugs.contains(&"ngc-test".to_string()),
        "kept_slugs = {:?}",
        kept_slugs
    );
    assert!(
        !kept_slugs.contains(&"pgc-far".to_string()),
        "pgc-far should be out of FOV"
    );
    assert!(outcome.kept >= 2);
}

#[tokio::test]
async fn identify_short_circuits_when_photo_missing_solve_data() {
    let (pool, _url) = fresh_db().await;

    let owner = Uuid::new_v4();
    sqlx::query("insert into users (id, display_name) values ($1, 'tester')")
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();

    // Photo with no solve data (ra_deg null).
    let photo_id = Uuid::new_v4();
    sqlx::query(
        r#"insert into photos (id, owner_id, storage_key, original_name, mime, width, height)
           values ($1, $2, 'k', 'n', 'image/jpeg', 1000, 1000)"#,
    )
    .bind(photo_id)
    .bind(owner)
    .execute(&pool)
    .await
    .unwrap();

    let mut tx = pool.begin().await.unwrap();
    let outcome = identify(photo_id, &mut tx).await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(outcome.found, 0);
    assert_eq!(outcome.kept, 0);
}
