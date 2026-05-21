//! Integration tests for GET /api/equipment/catalog (catalog-v2 browse
//! endpoint). Covers facet counts, brand filter, search, and sort.
//!
//! The handler is public — no auth required. We seed rows directly via
//! sqlx::query! against the testcontainer pool, then exercise the
//! axum Router via the `tower::ServiceExt::oneshot` pattern shared
//! across the rest of the integration suite.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use axum::{body::Body, http::Request};
use serde_json::Value;
use tower::ServiceExt;

async fn get_json(app: axum::Router, uri: &str) -> Value {
    let resp = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "expected 200 for {uri}");
    let bytes = axum::body::to_bytes(resp.into_body(), 1_000_000)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Insert a telescope item + matching telescope_specs row. Returns the
/// item id so individual tests can chain further inserts off it.
async fn seed_telescope(
    pool: &sqlx::PgPool,
    brand: &str,
    model: &str,
    aperture_mm: i32,
    focal_length_mm: i32,
    design: &str,
    usage_count: i32,
) -> uuid::Uuid {
    let display_name = format!("{brand} {model}");
    let canonical = display_name.to_lowercase();
    let item_id = sqlx::query_scalar!(
        r#"insert into equipment_items
            (kind, canonical_name, display_name, usage_count, status, approved_at,
             brand, model)
            values ('telescope', $1, $2, $3, 'approved', now(), $4, $5)
            returning id"#,
        canonical,
        display_name,
        usage_count,
        brand,
        model,
    )
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query!(
        r#"insert into telescope_specs (item_id, design, aperture_mm, focal_length_mm)
           values ($1, $2, $3, $4)"#,
        item_id,
        design,
        aperture_mm,
        focal_length_mm,
    )
    .execute(pool)
    .await
    .unwrap();
    item_id
}

/// Sanity: validate kind 422s on invalid input.
#[tokio::test]
async fn rejects_invalid_kind() {
    let (app, _pool) = common::make_app_and_pool().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/equipment/catalog?kind=bogus")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 422);
}

/// GET ?kind=telescope returns the items plus brand + design facet
/// counts grouped from the seeded data.
#[tokio::test]
async fn returns_items_and_facets() {
    let (app, pool) = common::make_app_and_pool().await;
    seed_telescope(
        &pool,
        "Sky-Watcher",
        "Esprit 100 ED",
        100,
        550,
        "refractor_apo",
        5,
    )
    .await;
    seed_telescope(
        &pool,
        "Sky-Watcher",
        "Esprit 120 ED",
        120,
        840,
        "refractor_apo",
        2,
    )
    .await;
    seed_telescope(&pool, "Celestron", "EdgeHD 8", 203, 2032, "sct", 7).await;
    seed_telescope(
        &pool,
        "Takahashi",
        "FSQ-106EDX4",
        106,
        530,
        "refractor_apo",
        1,
    )
    .await;

    let v = get_json(app, "/api/equipment/catalog?kind=telescope").await;
    let items = v["items"].as_array().unwrap();
    assert_eq!(items.len(), 4, "all four telescopes returned");
    assert_eq!(v["total"], 4);

    let brands = v["facets"]["brands"].as_array().unwrap();
    // Brands sorted by count desc — Sky-Watcher (2) before Celestron (1),
    // Takahashi (1). Tied count breaks alphabetically.
    assert_eq!(brands[0]["value"], "Sky-Watcher");
    assert_eq!(brands[0]["count"], 2);

    let designs = v["facets"]["designs"].as_array().unwrap();
    let refractor_apo = designs
        .iter()
        .find(|d| d["value"] == "refractor_apo")
        .unwrap();
    assert_eq!(refractor_apo["count"], 3);
    let sct = designs.iter().find(|d| d["value"] == "sct").unwrap();
    assert_eq!(sct["count"], 1);
}

/// `sort=aperture_desc` orders telescopes by aperture descending.
#[tokio::test]
async fn sort_by_aperture_desc_works_for_telescopes() {
    let (app, pool) = common::make_app_and_pool().await;
    seed_telescope(&pool, "ZWO", "FF80 APO", 80, 480, "refractor_apo", 0).await;
    seed_telescope(&pool, "Celestron", "EdgeHD 11", 280, 2800, "sct", 0).await;
    seed_telescope(
        &pool,
        "Sky-Watcher",
        "Esprit 100",
        100,
        550,
        "refractor_apo",
        0,
    )
    .await;

    let v = get_json(
        app,
        "/api/equipment/catalog?kind=telescope&sort=aperture_desc",
    )
    .await;
    let apertures: Vec<i64> = v["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|it| it["specs"]["aperture_mm"].as_i64().unwrap())
        .collect();
    assert_eq!(
        apertures,
        vec![280, 100, 80],
        "sorted descending by aperture"
    );
}

/// `brand=` filter narrows the items but the OTHER brand buckets
/// stay visible in the facet block.
#[tokio::test]
async fn brand_filter_narrows_results() {
    let (app, pool) = common::make_app_and_pool().await;
    seed_telescope(
        &pool,
        "Sky-Watcher",
        "Esprit 100",
        100,
        550,
        "refractor_apo",
        5,
    )
    .await;
    seed_telescope(
        &pool,
        "Sky-Watcher",
        "Esprit 120",
        120,
        840,
        "refractor_apo",
        2,
    )
    .await;
    seed_telescope(&pool, "Celestron", "EdgeHD 8", 203, 2032, "sct", 7).await;
    seed_telescope(&pool, "ZWO", "FF80", 80, 480, "refractor_apo", 1).await;

    let v = get_json(
        app,
        "/api/equipment/catalog?kind=telescope&brand=Sky-Watcher",
    )
    .await;
    assert_eq!(v["total"], 2, "Sky-Watcher has 2 telescopes");
    for it in v["items"].as_array().unwrap() {
        assert_eq!(it["brand"], "Sky-Watcher");
    }
    // Facets stay un-narrowed: all 3 brands should still appear.
    let brand_values: Vec<String> = v["facets"]["brands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["value"].as_str().unwrap().to_string())
        .collect();
    assert!(brand_values.contains(&"Sky-Watcher".to_string()));
    assert!(brand_values.contains(&"Celestron".to_string()));
    assert!(brand_values.contains(&"ZWO".to_string()));
}

/// Search matches brand + model + variant concatenation, not just
/// canonical_name / display_name.
#[tokio::test]
async fn search_matches_brand_model_concat() {
    let (app, pool) = common::make_app_and_pool().await;
    // Seed a row with a model name that wouldn't match by display_name
    // alone if the user typed the brand last (e.g. "esprit").
    seed_telescope(
        &pool,
        "Sky-Watcher",
        "Esprit 100 ED",
        100,
        550,
        "refractor_apo",
        0,
    )
    .await;
    seed_telescope(&pool, "Celestron", "EdgeHD 8", 203, 2032, "sct", 0).await;

    let v = get_json(
        app.clone(),
        "/api/equipment/catalog?kind=telescope&q=esprit",
    )
    .await;
    assert_eq!(v["total"], 1, "search matched 1 esprit telescope");
    assert_eq!(v["items"][0]["model"], "Esprit 100 ED");

    let v2 = get_json(app, "/api/equipment/catalog?kind=telescope&q=Sky-Watcher").await;
    assert_eq!(v2["total"], 1, "brand prefix match works too");
}

/// `min_aperture` / `max_aperture` restrict to the range. Other kinds
/// ignore the param (no-op) — verified via a camera query.
#[tokio::test]
async fn aperture_range_filters_telescopes() {
    let (app, pool) = common::make_app_and_pool().await;
    seed_telescope(
        &pool,
        "Sky-Watcher",
        "Esprit 80",
        80,
        400,
        "refractor_apo",
        0,
    )
    .await;
    seed_telescope(
        &pool,
        "Sky-Watcher",
        "Esprit 100",
        100,
        550,
        "refractor_apo",
        0,
    )
    .await;
    seed_telescope(
        &pool,
        "Sky-Watcher",
        "Esprit 120",
        120,
        840,
        "refractor_apo",
        0,
    )
    .await;

    let v = get_json(
        app,
        "/api/equipment/catalog?kind=telescope&min_aperture=90&max_aperture=110",
    )
    .await;
    assert_eq!(v["total"], 1, "only 100mm telescope matches");
    assert_eq!(v["items"][0]["specs"]["aperture_mm"], 100);
}

/// `kind=filter` returns the filter_types facet and items with their
/// FilterSpecs payload populated.
#[tokio::test]
async fn filter_kind_returns_filter_facets() {
    let (app, pool) = common::make_app_and_pool().await;
    let item_id = sqlx::query_scalar!(
        r#"insert into equipment_items
            (kind, canonical_name, display_name, usage_count, status, approved_at,
             brand, model)
            values ('filter', 'astronomik h-alpha 6nm 2"', 'Astronomik H-alpha 6nm 2"',
                    3, 'approved', now(), 'Astronomik', 'H-alpha 6nm 2"')
            returning id"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query!(
        r#"insert into filter_specs (item_id, filter_type, bandwidth_nm, size, mounted)
           values ($1, 'h_alpha', 6.0, '2in', true)"#,
        item_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    let v = get_json(app, "/api/equipment/catalog?kind=filter").await;
    assert_eq!(v["total"], 1);
    let filter_types = v["facets"]["filter_types"].as_array().unwrap();
    assert_eq!(filter_types[0]["value"], "h_alpha");
    assert_eq!(filter_types[0]["count"], 1);
    // Telescope-specific facets must be absent for non-telescope kinds.
    assert!(v["facets"]["designs"].is_null());
}

/// `brand=Unknown` (the human-readable label rendered in the facet
/// sidebar for rows where `brand=''`) round-trips to the empty-string
/// DB value rather than literal "Unknown". Without this mapping the
/// Unknown checkbox would filter to zero matches.
#[tokio::test]
async fn brand_unknown_label_filters_empty_string_rows() {
    let (app, pool) = common::make_app_and_pool().await;

    // Two rows with brand='' (unknown) plus one with a real brand.
    let display1 = "Mystery Telescope";
    sqlx::query!(
        r#"insert into equipment_items
            (kind, canonical_name, display_name, usage_count, status, approved_at,
             brand, model)
            values ('telescope', $1, $2, 0, 'approved', now(), '', $2)"#,
        display1.to_lowercase(),
        display1,
    )
    .execute(&pool)
    .await
    .unwrap();
    let display2 = "Another Unknown";
    sqlx::query!(
        r#"insert into equipment_items
            (kind, canonical_name, display_name, usage_count, status, approved_at,
             brand, model)
            values ('telescope', $1, $2, 0, 'approved', now(), '', $2)"#,
        display2.to_lowercase(),
        display2,
    )
    .execute(&pool)
    .await
    .unwrap();
    seed_telescope(
        &pool,
        "Sky-Watcher",
        "Esprit 100",
        100,
        550,
        "refractor_apo",
        0,
    )
    .await;

    let v = get_json(app, "/api/equipment/catalog?kind=telescope&brand=Unknown").await;
    assert_eq!(
        v["total"], 2,
        "Unknown filter should match the two empty-brand rows"
    );
    for it in v["items"].as_array().unwrap() {
        assert_eq!(
            it["brand"], "",
            "matched items must be the empty-brand ones"
        );
    }
}

/// Regression: hyphenated brand canonical_names round-trip cleanly
/// through the catalog browse → detail navigation. The browse page
/// builds a URL slug via `canonical_name.replace(/\s+/g, '-')`. For
/// `brand="Sky-Watcher" model="Esprit"`, canonical_name is
/// `"sky-watcher esprit"` (one hyphen + one space); the resulting
/// slug `"sky-watcher-esprit"` collapses both separators. The
/// discovery handler used to reverse that with a strict
/// `replace('-', ' ')`, producing `"sky watcher esprit"` — no match
/// in the DB.
///
/// The fix lives in `discovery::equipment::get`: the canonical
/// lookup now matches against both forms (`canonical_name = $slug`
/// OR `replace(canonical_name, '-', ' ') = $slug`). The same
/// tolerance extends to the per-kind photo_count queries below.
#[tokio::test]
async fn hyphenated_brand_slug_resolves_to_canonical_name() {
    let (app, pool) = common::make_app_and_pool().await;
    sqlx::query!(
        r#"insert into equipment_items
            (kind, canonical_name, display_name, usage_count, status, approved_at,
             brand, model)
            values ('telescope', 'sky-watcher esprit 100 ed',
                    'Sky-Watcher Esprit 100 ED', 0, 'approved', now(),
                    'Sky-Watcher', 'Esprit 100 ED')"#
    )
    .execute(&pool)
    .await
    .unwrap();

    // The catalog browse page builds this exact URL by replacing all
    // whitespace runs in canonical_name with hyphens. The discovery
    // handler must resolve it back to the canonical row.
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/equipment/telescope/sky-watcher-esprit-100-ed")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "hyphenated brand slug should resolve to the canonical row"
    );
}

/// Pagination: limit + page returns the right slice and `total` stays
/// the full count.
#[tokio::test]
async fn pagination_works() {
    let (app, pool) = common::make_app_and_pool().await;
    for i in 0..5 {
        seed_telescope(
            &pool,
            "Sky-Watcher",
            &format!("Esprit {}", 80 + i * 10),
            80 + i * 10,
            400 + i * 50,
            "refractor_apo",
            i,
        )
        .await;
    }

    let v = get_json(
        app.clone(),
        "/api/equipment/catalog?kind=telescope&limit=2&page=0",
    )
    .await;
    assert_eq!(v["items"].as_array().unwrap().len(), 2);
    assert_eq!(v["total"], 5);

    let v2 = get_json(app, "/api/equipment/catalog?kind=telescope&limit=2&page=2").await;
    assert_eq!(
        v2["items"].as_array().unwrap().len(),
        1,
        "last page has 1 of 5"
    );
    assert_eq!(v2["total"], 5);
}
