//! Integration tests for migration 0022 (equipment catalog v2):
//! brand/model split on the header, completeness columns on every
//! specs table, and the new `guiding_specs` sub-table.
//!
//! The testcontainer harness runs every migration on startup so by the
//! time the test connects, the schema already includes brand/model.
//! That means "migration_backfills_known_brands" verifies the backfill
//! CASE *expression* end-to-end via a manual UPDATE against a fresh row
//! — same logic as the migration, exercised in isolation. The migration
//! against an empty staging DB is otherwise unobservable from a test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::{
    body::Body,
    http::{Request, header},
};
use http_body_util::BodyExt as _;
use tower::ServiceExt;

/// Manual re-application of the migration's brand/model backfill CASE
/// expression. Kept in lock-step with `0022_equipment_catalog_v2.sql`;
/// edits to the whitelist must be mirrored here. Split into two
/// statements because prepared statements only accept one command.
const BACKFILL_UPDATE_BRAND_MODEL: &str = r#"
update equipment_items set
  brand = case
    when lower(display_name) like 'sky-watcher %' or lower(display_name) like 'skywatcher %' or lower(display_name) like 'sky watcher %'
      then 'Sky-Watcher'
    when lower(display_name) like 'zwo %' then 'ZWO'
    when lower(display_name) like 'celestron %' then 'Celestron'
    when lower(display_name) like 'qhy%' then 'QHY'
    when lower(display_name) like 'baader planetarium %' then 'Baader'
    when lower(display_name) like 'baader %' then 'Baader'
    else ''
  end,
  model = case
    when lower(display_name) like 'baader planetarium %' then substring(display_name from 20)
    when lower(display_name) like 'sky-watcher %' then substring(display_name from 13)
    when lower(display_name) like 'skywatcher %'  then substring(display_name from 12)
    when lower(display_name) like 'sky watcher %' then substring(display_name from 13)
    when lower(display_name) like 'zwo %'         then substring(display_name from 5)
    when lower(display_name) like 'celestron %'   then substring(display_name from 11)
    when lower(display_name) like 'qhy%'          then display_name
    when lower(display_name) like 'baader %'      then substring(display_name from 8)
    else display_name
  end
where id = $1
"#;

const BACKFILL_UPDATE_TRIM: &str =
    "update equipment_items set brand = trim(brand), model = trim(model) where id = $1";

async fn run_backfill(pool: &sqlx::PgPool, item_id: uuid::Uuid) {
    sqlx::query(BACKFILL_UPDATE_BRAND_MODEL)
        .bind(item_id)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(BACKFILL_UPDATE_TRIM)
        .bind(item_id)
        .execute(pool)
        .await
        .unwrap();
}

/// The backfill correctly splits "Sky-Watcher Esprit 100 ED" into
/// brand='Sky-Watcher' / model='Esprit 100 ED'.
#[tokio::test]
async fn migration_backfills_known_brands() {
    let (_app, pool) = common::make_app_and_pool().await;

    // Insert a freetext row (matches what upsert.rs writes today —
    // brand='', model=display_name). Then re-run the backfill UPDATE
    // to verify the CASE expression splits it correctly.
    let item_id: uuid::Uuid = sqlx::query_scalar!(
        r#"insert into equipment_items
                (kind, canonical_name, display_name, status, approved_at,
                 brand, model)
            values ('telescope', 'sky-watcher esprit 100 ed',
                    'Sky-Watcher Esprit 100 ED', 'approved', now(),
                    '', 'Sky-Watcher Esprit 100 ED')
            returning id"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    run_backfill(&pool, item_id).await;

    let r = sqlx::query!(
        "select brand, model, variant from equipment_items where id = $1",
        item_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(r.brand, "Sky-Watcher", "brand should be split off");
    assert_eq!(
        r.model, "Esprit 100 ED",
        "model should be the remainder after the brand"
    );
    assert!(r.variant.is_none(), "no variant for this name");
}

/// Names that don't match any brand prefix on the whitelist fall back
/// to brand='' and model=display_name.
#[tokio::test]
async fn unknown_brand_falls_back_to_empty() {
    let (_app, pool) = common::make_app_and_pool().await;

    let item_id: uuid::Uuid = sqlx::query_scalar!(
        r#"insert into equipment_items
                (kind, canonical_name, display_name, status, approved_at,
                 brand, model)
            values ('telescope', 'unknown telescope x',
                    'Unknown Telescope X', 'approved', now(),
                    '', 'Unknown Telescope X')
            returning id"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    run_backfill(&pool, item_id).await;

    let r = sqlx::query!(
        "select brand, model from equipment_items where id = $1",
        item_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(r.brand, "", "unknown brand should be empty");
    assert_eq!(
        r.model, "Unknown Telescope X",
        "model should be the whole display_name when brand unknown"
    );
}

/// Create a catalog item of kind='guiding', attach a `guiding_specs`
/// row via direct INSERT, then verify the GET endpoint round-trips
/// the new fields under `specs.kind = 'guiding'`.
#[tokio::test]
async fn guiding_specs_insert_and_read() {
    let (app, pool) = common::make_app_and_pool().await;
    let cookie = common::signup_and_cookie(&app, &pool, "guider@example.com", "guider1").await;

    let item_id: uuid::Uuid = sqlx::query_scalar!(
        r#"insert into equipment_items
                (kind, canonical_name, display_name, status, approved_at,
                 brand, model)
            values ('guiding', 'zwo asi120mm via oag',
                    'ZWO ASI120MM via OAG', 'approved', now(),
                    'ZWO', 'ASI120MM via OAG')
            returning id"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    sqlx::query!(
        r#"insert into guiding_specs
            (item_id, setup_kind, guide_focal_mm, guide_aperture_mm, guide_camera)
            values ($1, 'oag', 250, 60, 'ZWO ASI120MM')"#,
        item_id
    )
    .execute(&pool)
    .await
    .unwrap();

    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/equipment/items/{item_id}"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let body: serde_json::Value =
        serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap();

    assert_eq!(body["kind"], "guiding");
    assert_eq!(body["brand"], "ZWO");
    assert_eq!(body["model"], "ASI120MM via OAG");
    assert_eq!(body["specs"]["kind"], "guiding");
    assert_eq!(body["specs"]["setup_kind"], "oag");
    assert_eq!(body["specs"]["guide_focal_mm"], 250);
    assert_eq!(body["specs"]["guide_aperture_mm"], 60);
    assert_eq!(body["specs"]["guide_camera"], "ZWO ASI120MM");
}
