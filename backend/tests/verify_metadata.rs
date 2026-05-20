//! Integration tests for Task 33: verify-step PUT extended with
//! target / tags / category / equipment fields.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use astrophoto::{Config, db, http, storage::MemoryStorage};
use axum::{
    Router,
    body::Body,
    http::{Request, header},
};
use sqlx::PgPool;
use testcontainers::ImageExt;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;
use tower::ServiceExt;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

struct H {
    app: Router,
    pool: PgPool,
    _pg: testcontainers::ContainerAsync<PgImage>,
}

#[allow(clippy::unwrap_used)]
fn config_for(url: &str) -> Config {
    Config {
        bind: "127.0.0.1:0".into(),
        log: "info".into(),
        database_url: url.into(),
        session_domain: "localhost".into(),
        session_secure: false,
        public_base_url: "http://localhost:8080".into(),
        s3_endpoint: None,
        s3_region: "us-east-1".into(),
        s3_bucket: "x".into(),
        s3_access_key: "a".into(),
        s3_secret_key: "s".into(),
        s3_path_style: true,
        cdn_base_url: "http://localhost:0/cdn".into(),
        cdn_local_fallback: false,
        cors_origin: None,
        oauth_google_client_id: String::new(),
        oauth_google_client_secret: String::new(),
        oauth_google_redirect_url: String::new(),
        smtp_host: "unused-in-tests".into(),
        smtp_port: 1025,
        smtp_user: String::new(),
        smtp_pass: String::new(),
        mail_from: "test <test@astrophoto.local>".into(),
        smtp_tls: false,
        platesolve_base_url: None,
        platesolve_api_key: None,
        platesolve_timeout_secs: 90,
    }
}

async fn harness() -> H {
    let pg = PgImage::default()
        .with_tag("16-alpine")
        .start()
        .await
        .unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let storage = Arc::new(MemoryStorage::new());
    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    let app = http::router(
        pool.clone(),
        config_for(&url),
        storage,
        Arc::new(mailer),
        None,
    );
    H { app, pool, _pg: pg }
}

impl H {
    /// POST /api/auth/signup, mark verified, log in, return session cookie string.
    async fn signup(&self, email: &str, password: &str) -> String {
        let handle = email.split('@').next().unwrap_or("user").to_string();
        let body = serde_json::json!({
            "email": email,
            "password": password,
            "display_name": handle,
            "handle": handle,
        });
        let resp = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/signup")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 202, "signup failed for {email}");

        // Mark user verified.
        sqlx::query!(
            "update users set email_verified_at = now() where email = $1",
            email
        )
        .execute(&self.pool)
        .await
        .unwrap();

        // Log in to get session cookie.
        let login_body = serde_json::json!({"email": email, "password": password});
        let login_resp = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(login_body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            login_resp.status(),
            200,
            "login must succeed after signup for {email}"
        );
        login_resp
            .headers()
            .get("set-cookie")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }

    /// Look up user_id via GET /api/auth/me.
    async fn user_id(&self, cookie: &str) -> Uuid {
        let resp = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/auth/me")
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        Uuid::parse_str(v["id"].as_str().unwrap()).unwrap()
    }

    /// Insert a pending photo row directly via SQL. Returns photo id.
    async fn seed_photo(&self, cookie: &str) -> Uuid {
        let user_id = self.user_id(cookie).await;
        let photo_id = Uuid::new_v4();
        let storage_key = format!("originals/test-{photo_id}");
        let short_id = format!("T{}", &photo_id.to_string().replace('-', "")[..7]);
        sqlx::query!(
            r#"
            insert into photos
                (id, owner_id, storage_key, original_name, bytes, mime,
                 short_id, status, last_step, original_uploaded_at)
            values ($1, $2, $3, 'test.jpg', 1000, 'image/jpeg',
                    $4, 'ready', 'upload', now())
            "#,
            photo_id,
            user_id,
            storage_key,
            short_id,
        )
        .execute(&self.pool)
        .await
        .unwrap();
        photo_id
    }

    /// GET a path with a session cookie. Returns the parsed JSON body.
    async fn get_json(&self, path: &str, cookie: &str) -> serde_json::Value {
        let resp = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(path)
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(resp.status().is_success(), "GET {path} → {}", resp.status());
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// PUT /api/photos/:id with a JSON body. Returns HTTP status.
    async fn put_status(&self, path: &str, body: &serde_json::Value, cookie: &str) -> u16 {
        let resp = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(path)
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, cookie)
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        resp.status().as_u16()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// PUT with full body: category, scope, mount, filters, guiding, tags, target.
/// Asserts all side effects are written.
#[tokio::test]
async fn full_verify_put_writes_all_fields() {
    let h = harness().await;
    let cookie = h.signup("marie@example.com", "correcthorsebattery").await;
    let photo_id = h.seed_photo(&cookie).await;

    let body = serde_json::json!({
        "category": "dso",
        "target": "M31",
        "scope": "Celestron EdgeHD 8",
        "mount": "Sky-Watcher EQ6-R",
        "filters": "Baader Ha 7nm",
        "guiding": "ZWO OAG + ASI120MM",
        "tags": ["andromeda", "galaxy", "narrowband"],
        "last_step": "verify"
    });

    let status = h
        .put_status(&format!("/api/photos/{photo_id}"), &body, &cookie)
        .await;
    assert_eq!(status, 200, "expected 200, got {status}");

    // ---- photos row ----
    let row = sqlx::query!(
        "select category, scope, mount, filters, guiding from photos where id = $1",
        photo_id
    )
    .fetch_one(&h.pool)
    .await
    .unwrap();

    assert_eq!(row.category.as_deref(), Some("dso"));
    assert_eq!(row.scope.as_deref(), Some("Celestron EdgeHD 8"));
    assert_eq!(row.mount.as_deref(), Some("Sky-Watcher EQ6-R"));
    assert_eq!(row.filters.as_deref(), Some("Baader Ha 7nm"));
    assert_eq!(row.guiding.as_deref(), Some("ZWO OAG + ASI120MM"));

    // ---- photo_targets: M31 must have matched the seeded m31 row ----
    let target_count: i64 = sqlx::query_scalar!(
        r#"
        select count(*) from photo_targets pt
          join targets t on t.id = pt.target_id
         where pt.photo_id = $1
           and t.slug = 'm31'
           and pt.is_primary = true
        "#,
        photo_id
    )
    .fetch_one(&h.pool)
    .await
    .unwrap()
    .unwrap_or(0);
    assert_eq!(target_count, 1, "expected photo_targets row for m31");

    // ---- photo_tags: 3 rows ----
    let tag_count: i64 = sqlx::query_scalar!(
        "select count(*) from photo_tags where photo_id = $1",
        photo_id
    )
    .fetch_one(&h.pool)
    .await
    .unwrap()
    .unwrap_or(0);
    assert_eq!(tag_count, 3, "expected 3 photo_tags rows");

    // ---- equipment_items: telescope, mount, filter each present ----
    for (kind, canonical) in [
        ("telescope", "celestron edgehd 8"),
        ("mount", "sky-watcher eq6-r"),
        ("filter", "baader ha 7nm"),
    ] {
        let eq_count: i64 = sqlx::query_scalar!(
            "select count(*) from equipment_items where kind = $1 and canonical_name = $2",
            kind,
            canonical
        )
        .fetch_one(&h.pool)
        .await
        .unwrap()
        .unwrap_or(0);
        assert_eq!(eq_count, 1, "expected equipment_items row for kind={kind}");
    }
    // Guiding is now a first-class catalog kind: the verify-form fan-out
    // creates an equipment_items row alongside camera/telescope/mount/filter.
    let guiding_count: i64 =
        sqlx::query_scalar!("select count(*) from equipment_items where kind = 'guiding'")
            .fetch_one(&h.pool)
            .await
            .unwrap()
            .unwrap_or(0);
    assert_eq!(
        guiding_count, 1,
        "guiding should now create an equipment_items row (catalog kind, migration 0017)"
    );
}

/// Sending an invalid category returns 422 / 400 (validation error).
#[tokio::test]
async fn invalid_category_rejected() {
    let h = harness().await;
    let cookie = h.signup("bob@example.com", "correcthorsebattery").await;
    let photo_id = h.seed_photo(&cookie).await;

    let body = serde_json::json!({ "category": "supernova" });
    let status = h
        .put_status(&format!("/api/photos/{photo_id}"), &body, &cookie)
        .await;
    assert!(
        status == 400 || status == 422,
        "expected 400/422, got {status}"
    );
}

/// Sending more than 8 tags returns a validation error.
#[tokio::test]
async fn too_many_tags_rejected() {
    let h = harness().await;
    let cookie = h.signup("carol@example.com", "correcthorsebattery").await;
    let photo_id = h.seed_photo(&cookie).await;

    let tags: Vec<String> = (1..=9).map(|i| format!("tag{i}")).collect();
    let body = serde_json::json!({ "tags": tags });
    let status = h
        .put_status(&format!("/api/photos/{photo_id}"), &body, &cookie)
        .await;
    assert!(
        status == 400 || status == 422,
        "expected 400/422, got {status}"
    );
}

/// Another user cannot PUT to a photo they don't own.
#[tokio::test]
async fn other_user_forbidden() {
    let h = harness().await;
    let owner = h.signup("dave@example.com", "correcthorsebattery").await;
    let other = h.signup("eve@example.com", "correcthorsebattery").await;
    let photo_id = h.seed_photo(&owner).await;

    let body = serde_json::json!({ "category": "dso" });
    let status = h
        .put_status(&format!("/api/photos/{photo_id}"), &body, &other)
        .await;
    assert_eq!(status, 403, "expected 403, got {status}");
}

/// PUT with the migration 0013 extended fields persists them and the
/// follow-up GET echoes them back. Round-trips both new acquisition
/// fields and the (already-stored-but-newly-exposed) RA/Dec.
#[tokio::test]
async fn extended_exif_round_trip() {
    let h = harness().await;
    let cookie = h.signup("nora@example.com", "correcthorsebattery").await;
    let photo_id = h.seed_photo(&cookie).await;

    let body = serde_json::json!({
        "aperture_f": 5.0,
        "gain": 100,
        "sensor_temp_c": -10.5,
        "sessions": 4,
        "ra_deg": 314.7,
        "dec_deg": 44.33,
    });

    let status = h
        .put_status(&format!("/api/photos/{photo_id}"), &body, &cookie)
        .await;
    assert_eq!(status, 200, "expected 200, got {status}");

    let row = sqlx::query!(
        "select aperture_f, gain, sensor_temp_c, sessions, ra_deg, dec_deg \
         from photos where id = $1",
        photo_id
    )
    .fetch_one(&h.pool)
    .await
    .unwrap();

    assert!((row.aperture_f.unwrap() - 5.0).abs() < 0.001);
    assert_eq!(row.gain, Some(100));
    assert!((row.sensor_temp_c.unwrap() - (-10.5)).abs() < 0.001);
    assert_eq!(row.sessions, Some(4));
    assert!((row.ra_deg.unwrap() - 314.7).abs() < 0.0001);
    assert!((row.dec_deg.unwrap() - 44.33).abs() < 0.0001);
}

/// Clearing an extended field is a JSON `null` in the patch.
#[tokio::test]
async fn extended_exif_clearing() {
    let h = harness().await;
    let cookie = h.signup("ophelia@example.com", "correcthorsebattery").await;
    let photo_id = h.seed_photo(&cookie).await;

    // Set values.
    let set_body = serde_json::json!({
        "aperture_f": 4.0,
        "gain": 50,
    });
    h.put_status(&format!("/api/photos/{photo_id}"), &set_body, &cookie)
        .await;

    // Clear by sending JSON nulls.
    let clear_body = serde_json::json!({
        "aperture_f": null,
        "gain": null,
    });
    let status = h
        .put_status(&format!("/api/photos/{photo_id}"), &clear_body, &cookie)
        .await;
    assert_eq!(status, 200, "expected 200, got {status}");

    let row = sqlx::query!(
        "select aperture_f, gain from photos where id = $1",
        photo_id
    )
    .fetch_one(&h.pool)
    .await
    .unwrap();

    assert!(row.aperture_f.is_none(), "aperture_f should be cleared");
    assert!(row.gain.is_none(), "gain should be cleared");
}

/// PUT with no target sends no photo_targets row.
#[tokio::test]
async fn no_target_no_photo_targets_row() {
    let h = harness().await;
    let cookie = h.signup("frank@example.com", "correcthorsebattery").await;
    let photo_id = h.seed_photo(&cookie).await;

    let body = serde_json::json!({ "category": "nightscape" });
    let status = h
        .put_status(&format!("/api/photos/{photo_id}"), &body, &cookie)
        .await;
    assert_eq!(status, 200, "expected 200, got {status}");

    let count: i64 = sqlx::query_scalar!(
        "select count(*) from photo_targets where photo_id = $1",
        photo_id
    )
    .fetch_one(&h.pool)
    .await
    .unwrap()
    .unwrap_or(0);
    assert_eq!(count, 0, "expected no photo_targets rows");
}

/// PUT focal_modifier, GET it back, confirm equipment_items fan-out.
#[tokio::test]
async fn focal_modifier_round_trip_via_put_and_get() {
    let h = harness().await;
    let cookie = h.signup("zara@example.com", "correcthorsebattery").await;
    let photo_id = h.seed_photo(&cookie).await;

    // PUT focal_modifier
    let body = serde_json::json!({ "focal_modifier": "Antares 0.7x Reducer" });
    let status = h
        .put_status(&format!("/api/photos/{photo_id}"), &body, &cookie)
        .await;
    assert_eq!(status, 200, "expected 200, got {status}");

    // GET and confirm it round-trips
    let detail = h
        .get_json(&format!("/api/photos/{photo_id}"), &cookie)
        .await;
    assert_eq!(
        detail["focal_modifier"], "Antares 0.7x Reducer",
        "focal_modifier not returned in GET response"
    );

    // The fan-out should have created an equipment_items row.
    let count: i64 = sqlx::query_scalar!(
        "select count(*) from equipment_items \
         where kind = 'focal_modifier' and canonical_name = 'antares 0.7x reducer'"
    )
    .fetch_one(&h.pool)
    .await
    .unwrap()
    .unwrap_or(0);
    assert_eq!(count, 1, "expected equipment_items row for focal_modifier");
}
