//! Integration tests for Phase 8b: drafts, replace, my-photos stats,
//! visibility predicate. Phase 5 upload tests stay in `photos.rs`.

use std::sync::Arc;

use astrophoto::storage::{MemoryStorage, Storage};
use astrophoto::{Config, db, http};
use axum::{
    Router,
    body::Body,
    http::{Request, header},
};
use http_body_util::BodyExt as _;
use image::{DynamicImage, ImageFormat, RgbImage};
use sqlx::PgPool;
use std::io::Cursor;
use testcontainers::ImageExt;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;
use tower::ServiceExt;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Shared utilities (mirrored from photos.rs — not exposed as a crate item)
// ---------------------------------------------------------------------------

fn handle_from_email(email: &str) -> String {
    let local = email.split('@').next().unwrap_or("user");
    let mut h = local
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>();
    h = h.trim_matches('-').to_string();
    if h.len() < 3 {
        h = format!("t-{h}");
    }
    h
}

#[allow(clippy::expect_used)]
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

#[allow(clippy::unwrap_used)]
fn make_test_jpeg() -> Vec<u8> {
    let img = DynamicImage::ImageRgb8(RgbImage::from_fn(200, 150, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 64])
    }));
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, ImageFormat::Jpeg).unwrap();
    buf.into_inner()
}

// ---------------------------------------------------------------------------
// HTTP harness
// ---------------------------------------------------------------------------

/// Test harness holding a live Postgres testcontainer and an in-process axum
/// router. The container field keeps the container alive for the harness
/// lifetime; the `_` prefix suppresses the dead_code lint.
struct H {
    app: Router,
    pool: PgPool,
    /// Concrete MemoryStorage so tests can inspect S3 state directly.
    storage: Arc<MemoryStorage>,
    _pg: testcontainers::ContainerAsync<PgImage>,
}

/// Spin up a fresh Postgres container, run migrations, and build the axum
/// router with MemoryStorage and a test mailer.
#[allow(clippy::unwrap_used)]
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
    // Arc<MemoryStorage> coerces to Arc<dyn Storage> at the call site.
    let app = http::router(
        pool.clone(),
        config_for(&url),
        storage.clone(),
        Arc::new(mailer),
        None,
    );

    H {
        app,
        pool,
        storage,
        _pg: pg,
    }
}

impl H {
    /// Poll until `photos.status = 'ready'` for the given photo id.
    /// Panics if not reached within 10 s.
    #[allow(clippy::unwrap_used, clippy::panic)]
    async fn wait_for_ready(&self, id: Uuid) {
        for _ in 0..200 {
            let status = sqlx::query_scalar!("select status from photos where id = $1", id)
                .fetch_one(&self.pool)
                .await
                .unwrap();
            if status == "ready" {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        panic!("photo {id} did not reach status='ready' within 10 s");
    }

    /// POST /api/auth/signup, mark verified, login, and return the `set-cookie` header value.
    #[allow(clippy::unwrap_used)]
    async fn signup(&self, email: &str, password: &str, display_name: &str) -> String {
        let handle = handle_from_email(email);
        let body = serde_json::json!({
            "email": email,
            "password": password,
            "display_name": display_name,
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

        // Mark user verified so login works.
        sqlx::query!(
            "update users set email_verified_at = now() where email = $1",
            email
        )
        .execute(&self.pool)
        .await
        .unwrap();

        // Log in to obtain a session cookie.
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

    /// Seed a ready photo directly via SQL + MemoryStorage for the given user
    /// (identified by their session cookie). Returns the new photo id.
    ///
    /// The photo is seeded in `status='ready'` with `published_at IS NULL` so
    /// it behaves as a draft: visible only to the owner, not publicly listed.
    /// `wait_for_ready` calls on the returned id are instant no-ops.
    #[allow(clippy::unwrap_used)]
    async fn seed_photo(&self, cookie: &str) -> Uuid {
        let user_id = self.user_id(cookie).await;
        let photo_id = Uuid::new_v4();
        let storage_key = format!("originals/test-{photo_id}");
        // Use a unique short_id per photo to avoid collisions across tests.
        let short_id = format!("T{}", &photo_id.to_string().replace('-', "")[..7]);
        sqlx::query!(
            r#"
            insert into photos
                (id, owner_id, storage_key, original_name, bytes, mime,
                 short_id, status, last_step, original_uploaded_at)
            values ($1, $2, $3, 'draft.jpg', 1000, 'image/jpeg',
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
        // Seed a JPEG into storage so S3 checks in replace/delete tests work.
        let jpeg = make_test_jpeg();
        self.storage
            .put(&storage_key, "image/jpeg", bytes::Bytes::from(jpeg))
            .await
            .unwrap();
        photo_id
    }

    /// GET /api/auth/me with the given cookie, returns `user.id` as Uuid.
    #[allow(clippy::unwrap_used)]
    async fn user_id(&self, cookie: &str) -> Uuid {
        let body = self.get_json("/api/auth/me", Some(cookie)).await;
        Uuid::parse_str(body["id"].as_str().unwrap()).unwrap()
    }

    /// GET `path` optionally authenticated. Returns parsed JSON body.
    #[allow(clippy::unwrap_used)]
    async fn get_json(&self, path: &str, cookie: Option<&str>) -> serde_json::Value {
        let mut req = Request::builder().method("GET").uri(path);
        if let Some(c) = cookie {
            req = req.header(header::COOKIE, c);
        }
        let resp = self
            .app
            .clone()
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// GET `path` optionally authenticated. Returns HTTP status code only.
    #[allow(clippy::unwrap_used)]
    async fn get_status(&self, path: &str, cookie: Option<&str>) -> u16 {
        let mut req = Request::builder().method("GET").uri(path);
        if let Some(c) = cookie {
            req = req.header(header::COOKIE, c);
        }
        let resp = self
            .app
            .clone()
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap();
        resp.status().as_u16()
    }

    /// POST `path` with optional JSON body, optionally authenticated.
    /// Returns HTTP status code only.
    #[allow(clippy::unwrap_used, dead_code)]
    async fn post_status(
        &self,
        path: &str,
        body: Option<serde_json::Value>,
        cookie: Option<&str>,
    ) -> u16 {
        let mut req = Request::builder().method("POST").uri(path);
        if let Some(c) = cookie {
            req = req.header(header::COOKIE, c);
        }
        let (body_bytes, content_type) = match body {
            Some(v) => (v.to_string().into_bytes(), "application/json"),
            None => (Vec::new(), "application/json"),
        };
        req = req.header(header::CONTENT_TYPE, content_type);
        let resp = self
            .app
            .clone()
            .oneshot(req.body(Body::from(body_bytes)).unwrap())
            .await
            .unwrap();
        resp.status().as_u16()
    }

    /// PUT `path` with a JSON body, optionally authenticated.
    /// Returns HTTP status code only.
    #[allow(clippy::unwrap_used, dead_code)]
    async fn put_status(&self, path: &str, body: &serde_json::Value, cookie: Option<&str>) -> u16 {
        let mut req = Request::builder()
            .method("PUT")
            .uri(path)
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(c) = cookie {
            req = req.header(header::COOKIE, c);
        }
        let r = self
            .app
            .clone()
            .oneshot(req.body(Body::from(body.to_string())).unwrap())
            .await
            .unwrap();
        r.status().as_u16()
    }

    /// DELETE `path`, optionally authenticated. Returns HTTP status code only.
    #[allow(clippy::unwrap_used, dead_code)]
    async fn delete_status(&self, path: &str, cookie: Option<&str>) -> u16 {
        let mut req = Request::builder().method("DELETE").uri(path);
        if let Some(c) = cookie {
            req = req.header(header::COOKIE, c);
        }
        let resp = self
            .app
            .clone()
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap();
        resp.status().as_u16()
    }

    /// POST a fresh JPEG to `/api/photos/:id/replace`. Asserts 202.
    #[allow(clippy::unwrap_used, dead_code)]
    async fn replace_with_jpeg(&self, id: Uuid, cookie: &str) {
        let status = self.replace_status(id, cookie).await;
        assert!(status == 202, "replace returned {status}");
    }

    /// POST a fresh JPEG to `/api/photos/:id/replace`. Returns the HTTP status code.
    #[allow(clippy::unwrap_used, dead_code)]
    async fn replace_status(&self, id: Uuid, cookie: &str) -> u16 {
        let body = make_test_jpeg();
        let boundary = "----replaceboundary";
        let mut data = Vec::new();
        data.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        data.extend_from_slice(
            b"Content-Disposition: form-data; name=\"file\"; filename=\"replace.jpg\"\r\n",
        );
        data.extend_from_slice(b"Content-Type: image/jpeg\r\n\r\n");
        data.extend_from_slice(&body);
        data.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let req = Request::builder()
            .method("POST")
            .uri(format!("/api/photos/{id}/replace"))
            .header(header::COOKIE, cookie)
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(Body::from(data))
            .unwrap();
        let r = self.app.clone().oneshot(req).await.unwrap();
        r.status().as_u16()
    }
}

// ---------------------------------------------------------------------------
// Pre-existing DB-layer tests (Tasks 2 & 3)
// ---------------------------------------------------------------------------

#[allow(clippy::unwrap_used)]
async fn test_pool() -> (sqlx::PgPool, testcontainers::ContainerAsync<PgImage>) {
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
    (pool, pg)
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn is_visible_to_returns_true_for_published_to_anyone() {
    let (pool, _pg) = test_pool().await;
    let owner = Uuid::new_v4();
    let viewer = Uuid::new_v4();
    let owner_handle = format!("o-{}", &owner.to_string()[..8]);
    let viewer_handle = format!("v-{}", &viewer.to_string()[..8]);
    sqlx::query!(
        "insert into users (id, email, password_hash, display_name, handle)
         values ($1, $2, '', 'O', $5), ($3, $4, '', 'V', $6)",
        owner,
        format!("o-{owner}@e"),
        viewer,
        format!("v-{viewer}@e"),
        owner_handle,
        viewer_handle,
    )
    .execute(&pool)
    .await
    .unwrap();
    let photo_id = sqlx::query_scalar!(
        "insert into photos (owner_id, storage_key, original_name, bytes, mime,
                             status, published_at, original_uploaded_at, last_step, short_id)
         values ($1, 'k', 'n.jpg', 10, 'image/jpeg', 'ready', now(), now(), 'caption',
                 upper(left(replace(gen_random_uuid()::text, '-', ''), 8)))
         returning id",
        owner
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(
        astrophoto::photos::queries::is_visible_to(&pool, photo_id, Some(viewer))
            .await
            .unwrap()
    );
    assert!(
        astrophoto::photos::queries::is_visible_to(&pool, photo_id, None)
            .await
            .unwrap()
    );
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn is_visible_to_returns_false_for_draft_to_non_owner_and_anon() {
    let (pool, _pg) = test_pool().await;
    let owner = Uuid::new_v4();
    let viewer = Uuid::new_v4();
    let owner_handle = format!("o-{}", &owner.to_string()[..8]);
    let viewer_handle = format!("v-{}", &viewer.to_string()[..8]);
    sqlx::query!(
        "insert into users (id, email, password_hash, display_name, handle)
         values ($1, $2, '', 'O', $5), ($3, $4, '', 'V', $6)",
        owner,
        format!("o-{owner}@e"),
        viewer,
        format!("v-{viewer}@e"),
        owner_handle,
        viewer_handle,
    )
    .execute(&pool)
    .await
    .unwrap();
    let photo_id = sqlx::query_scalar!(
        "insert into photos (owner_id, storage_key, original_name, bytes, mime,
                             status, original_uploaded_at, last_step, short_id)
         values ($1, 'k', 'n.jpg', 10, 'image/jpeg', 'processing', now(), 'upload',
                 upper(left(replace(gen_random_uuid()::text, '-', ''), 8)))
         returning id",
        owner
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(
        !astrophoto::photos::queries::is_visible_to(&pool, photo_id, Some(viewer))
            .await
            .unwrap()
    );
    assert!(
        !astrophoto::photos::queries::is_visible_to(&pool, photo_id, None)
            .await
            .unwrap()
    );
    assert!(
        astrophoto::photos::queries::is_visible_to(&pool, photo_id, Some(owner))
            .await
            .unwrap()
    );
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn insert_processing_sets_last_step_upload_and_published_at_null() {
    let (pool, _pg) = test_pool().await;
    let owner = Uuid::new_v4();
    let owner_handle = format!("o-{}", &owner.to_string()[..8]);
    sqlx::query!(
        "insert into users (id, email, password_hash, display_name, handle)
         values ($1, $2, '', 'O', $3)",
        owner,
        format!("o-{owner}@e"),
        owner_handle,
    )
    .execute(&pool)
    .await
    .unwrap();

    let photo_id = astrophoto::photos::queries::insert_processing(
        &pool,
        owner,
        "k",
        "n.jpg",
        10,
        "image/jpeg",
        None,
        None,
    )
    .await
    .unwrap();

    let row = sqlx::query!(
        "select published_at, last_step, original_uploaded_at from photos where id = $1",
        photo_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(row.published_at.is_none());
    assert_eq!(row.last_step.as_deref(), Some("upload"));
    assert!(row.original_uploaded_at <= chrono::Utc::now());
}

// ---------------------------------------------------------------------------
// Task 4: drafts list endpoint tests
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn list_drafts_returns_only_callers_drafts() {
    let h = harness().await;
    let alice = h.signup("alice@e.com", "longenoughpw", "Alice").await;
    let bob = h.signup("bob@e.com", "longenoughpw", "Bob").await;

    let alice_draft_id = h.seed_photo(&alice).await;
    h.seed_photo(&bob).await; // draft for bob

    let body = h.get_json("/api/photos?drafts=true", Some(&alice)).await;
    let photos = body["photos"].as_array().unwrap();
    assert_eq!(photos.len(), 1, "alice sees only her own draft");
    assert_eq!(
        photos[0]["id"].as_str().unwrap(),
        alice_draft_id.to_string(),
        "the returned draft belongs to alice"
    );
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn list_drafts_with_cross_user_owner_id_is_rejected() {
    let h = harness().await;
    let alice = h.signup("alice@e.com", "longenoughpw", "Alice").await;
    let bob = h.signup("bob@e.com", "longenoughpw", "Bob").await;
    let bob_id = h.user_id(&bob).await;

    let status = h
        .get_status(
            &format!("/api/photos?drafts=true&owner_id={bob_id}"),
            Some(&alice),
        )
        .await;
    assert_eq!(status, 403);
}

// ---------------------------------------------------------------------------
// Task 5: photo detail visibility 404 + extended DTO
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn get_draft_returns_404_for_non_owner() {
    let h = harness().await;
    let alice = h.signup("alice@e.com", "longenoughpw", "Alice").await;
    let bob = h.signup("bob@e.com", "longenoughpw", "Bob").await;
    let photo_id = h.seed_photo(&alice).await;

    let status = h
        .get_status(&format!("/api/photos/{photo_id}"), Some(&bob))
        .await;
    assert_eq!(status, 404);

    let status_anon = h.get_status(&format!("/api/photos/{photo_id}"), None).await;
    assert_eq!(status_anon, 404);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn get_draft_returns_200_with_is_draft_for_owner() {
    let h = harness().await;
    let alice = h.signup("alice@e.com", "longenoughpw", "Alice").await;
    let photo_id = h.seed_photo(&alice).await;

    let body = h
        .get_json(&format!("/api/photos/{photo_id}"), Some(&alice))
        .await;
    assert_eq!(body["is_draft"], true);
    assert!(body["last_step"].as_str().is_some());
    assert!(body["replaced_at"].is_null());
}

// ---------------------------------------------------------------------------
// Task 6: engagement visibility on drafts
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn appreciation_count_on_draft_404s_for_non_owner() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let bob = h.signup("b@e.com", "longenoughpw", "Bob").await;
    let photo_id = h.seed_photo(&alice).await;

    let status = h
        .get_status(
            &format!("/api/photos/{photo_id}/appreciations/count"),
            Some(&bob),
        )
        .await;
    assert_eq!(status, 404);

    let status_anon = h
        .get_status(&format!("/api/photos/{photo_id}/appreciations/count"), None)
        .await;
    assert_eq!(status_anon, 404);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn appreciate_a_draft_returns_404() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let bob = h.signup("b@e.com", "longenoughpw", "Bob").await;
    let photo_id = h.seed_photo(&alice).await;

    let status = h
        .post_status(
            &format!("/api/photos/{photo_id}/appreciate"),
            None,
            Some(&bob),
        )
        .await;
    assert_eq!(status, 404);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn comment_list_on_draft_404s_for_non_owner() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let bob = h.signup("b@e.com", "longenoughpw", "Bob").await;
    let photo_id = h.seed_photo(&alice).await;

    let status = h
        .get_status(&format!("/api/photos/{photo_id}/comments"), Some(&bob))
        .await;
    assert_eq!(status, 404);
}

// ---------------------------------------------------------------------------
// Task 7: publish endpoint
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn publish_sets_published_at_and_last_step_caption() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let id = h.seed_photo(&alice).await;
    h.wait_for_ready(id).await;

    let status = h
        .post_status(&format!("/api/photos/{id}/publish"), None, Some(&alice))
        .await;
    assert_eq!(status, 200);
    let row = sqlx::query!("select published_at, last_step from photos where id=$1", id)
        .fetch_one(&h.pool)
        .await
        .unwrap();
    assert!(row.published_at.is_some());
    assert_eq!(row.last_step.as_deref(), Some("caption"));
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn publish_is_idempotent() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let id = h.seed_photo(&alice).await;
    h.wait_for_ready(id).await;
    h.post_status(&format!("/api/photos/{id}/publish"), None, Some(&alice))
        .await;
    let first = sqlx::query_scalar!("select published_at from photos where id=$1", id)
        .fetch_one(&h.pool)
        .await
        .unwrap()
        .unwrap();

    let status = h
        .post_status(&format!("/api/photos/{id}/publish"), None, Some(&alice))
        .await;
    assert_eq!(status, 200);
    let second = sqlx::query_scalar!("select published_at from photos where id=$1", id)
        .fetch_one(&h.pool)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        first, second,
        "publish must be idempotent — published_at unchanged"
    );
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn publish_403_for_non_owner() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let bob = h.signup("b@e.com", "longenoughpw", "Bob").await;
    let id = h.seed_photo(&alice).await;
    h.wait_for_ready(id).await;
    let status = h
        .post_status(&format!("/api/photos/{id}/publish"), None, Some(&bob))
        .await;
    assert_eq!(status, 403);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn publish_400_when_status_processing() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let alice_id = h.user_id(&alice).await;
    // Insert directly (bypassing seed_photo) so no background pipeline races.
    let id = sqlx::query_scalar!(
        "insert into photos (owner_id, storage_key, original_name, bytes, mime,
                             status, last_step, original_uploaded_at, short_id)
         values ($1, 'k', 'n.jpg', 10, 'image/jpeg', 'processing', 'upload', now(),
                 upper(left(replace(gen_random_uuid()::text, '-', ''), 8)))
         returning id",
        alice_id
    )
    .fetch_one(&h.pool)
    .await
    .unwrap();

    let status = h
        .post_status(&format!("/api/photos/{id}/publish"), None, Some(&alice))
        .await;
    assert_eq!(status, 400);
}

// ---------------------------------------------------------------------------
// Task 8: metadata partial-update PUT /api/photos/:id
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn put_metadata_works_on_draft_and_published() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let draft = h.seed_photo(&alice).await;
    h.wait_for_ready(draft).await;

    let body = serde_json::json!({
        "target": "M31",
        "caption": "first light",
        "iso": 1600,
        "last_step": "verify",
    });
    let status = h
        .put_status(&format!("/api/photos/{draft}"), &body, Some(&alice))
        .await;
    assert_eq!(status, 200);
    let row = sqlx::query!(
        "select target, caption, iso, last_step from photos where id=$1",
        draft
    )
    .fetch_one(&h.pool)
    .await
    .unwrap();
    assert_eq!(row.target.as_deref(), Some("M31"));
    assert_eq!(row.caption.as_deref(), Some("first light"));
    assert_eq!(row.iso, Some(1600));
    assert_eq!(row.last_step.as_deref(), Some("verify"));

    h.post_status(&format!("/api/photos/{draft}/publish"), None, Some(&alice))
        .await;
    let body2 = serde_json::json!({ "caption": "edited after publish" });
    let status = h
        .put_status(&format!("/api/photos/{draft}"), &body2, Some(&alice))
        .await;
    assert_eq!(status, 200);
    let row = sqlx::query!("select caption from photos where id=$1", draft)
        .fetch_one(&h.pool)
        .await
        .unwrap();
    assert_eq!(row.caption.as_deref(), Some("edited after publish"));
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn put_metadata_403_for_non_owner() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let bob = h.signup("b@e.com", "longenoughpw", "Bob").await;
    let id = h.seed_photo(&alice).await;
    let body = serde_json::json!({ "target": "hijack" });
    let status = h
        .put_status(&format!("/api/photos/{id}"), &body, Some(&bob))
        .await;
    assert_eq!(status, 403);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn put_metadata_explicit_null_clears_field() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let id = h.seed_photo(&alice).await;
    h.wait_for_ready(id).await;

    // Set target to a value
    let body = serde_json::json!({ "target": "M31" });
    h.put_status(&format!("/api/photos/{id}"), &body, Some(&alice))
        .await;
    let row = sqlx::query!("select target from photos where id=$1", id)
        .fetch_one(&h.pool)
        .await
        .unwrap();
    assert_eq!(row.target.as_deref(), Some("M31"));

    // Now explicitly clear it via JSON null
    let body = serde_json::json!({ "target": null });
    let status = h
        .put_status(&format!("/api/photos/{id}"), &body, Some(&alice))
        .await;
    assert_eq!(status, 200);
    let row = sqlx::query!("select target from photos where id=$1", id)
        .fetch_one(&h.pool)
        .await
        .unwrap();
    assert!(row.target.is_none(), "explicit null should clear the field");
}

// ---------------------------------------------------------------------------
// Task 9: pipeline_error capture
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn mark_failed_records_pipeline_error_string() {
    let (pool, _pg) = test_pool().await;
    let owner = Uuid::new_v4();
    let owner_handle = format!("o-{}", &owner.to_string()[..8]);
    sqlx::query!(
        "insert into users (id, email, password_hash, display_name, handle)
         values ($1, $2, '', 'O', $3)",
        owner,
        format!("o-{owner}@e"),
        owner_handle,
    )
    .execute(&pool)
    .await
    .unwrap();
    let id = sqlx::query_scalar!(
        "insert into photos (owner_id, storage_key, original_name, bytes, mime,
                             status, original_uploaded_at, last_step, short_id)
         values ($1, 'k', 'n.jpg', 10, 'image/jpeg', 'processing', now(), 'upload',
                 upper(left(replace(gen_random_uuid()::text, '-', ''), 8)))
         returning id",
        owner
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    astrophoto::photos::queries::mark_failed(&pool, id, "decode failed: bad jpeg")
        .await
        .unwrap();
    let row = sqlx::query!("select status, pipeline_error from photos where id=$1", id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.status, "failed");
    assert_eq!(
        row.pipeline_error.as_deref(),
        Some("decode failed: bad jpeg")
    );
}

// ---------------------------------------------------------------------------
// Task 10: Replace endpoint
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn replace_swaps_storage_key_keeps_metadata() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let id = h.seed_photo(&alice).await;
    h.wait_for_ready(id).await;
    h.put_status(
        &format!("/api/photos/{id}"),
        &serde_json::json!({"target":"M31","caption":"v1"}),
        Some(&alice),
    )
    .await;
    h.post_status(&format!("/api/photos/{id}/publish"), None, Some(&alice))
        .await;

    let key_before: String = sqlx::query_scalar!("select storage_key from photos where id=$1", id)
        .fetch_one(&h.pool)
        .await
        .unwrap();
    h.replace_with_jpeg(id, &alice).await;
    h.wait_for_ready(id).await;

    let row = sqlx::query!(
        "select storage_key, target, caption, replaced_at, published_at from photos where id=$1",
        id
    )
    .fetch_one(&h.pool)
    .await
    .unwrap();
    assert_ne!(row.storage_key, key_before, "master key swapped");
    assert_eq!(row.target.as_deref(), Some("M31"), "target preserved");
    assert_eq!(row.caption.as_deref(), Some("v1"), "caption preserved");
    assert!(row.replaced_at.is_some());
    assert!(row.published_at.is_some(), "published_at preserved");

    let pending: i64 = sqlx::query_scalar!(
        r#"select count(*) as "c!" from photo_pending_deletes where photo_id = $1"#,
        id
    )
    .fetch_one(&h.pool)
    .await
    .unwrap();
    assert_eq!(pending, 0, "pending deletes must be drained after replace");

    let thumb_count: i64 = sqlx::query_scalar!(
        r#"select count(*) as "c!" from thumbnails where photo_id = $1"#,
        id
    )
    .fetch_one(&h.pool)
    .await
    .unwrap();
    assert!(
        thumb_count > 0,
        "thumbnails must be regenerated after replace"
    );
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn replace_403_for_non_owner() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let bob = h.signup("b@e.com", "longenoughpw", "Bob").await;
    let id = h.seed_photo(&alice).await;
    h.wait_for_ready(id).await;
    let status = h.replace_status(id, &bob).await;
    assert_eq!(status, 403);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn replace_400_when_pipeline_busy() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let id = h.seed_photo(&alice).await;
    h.wait_for_ready(id).await;
    sqlx::query!("update photos set status='processing' where id=$1", id)
        .execute(&h.pool)
        .await
        .unwrap();
    let status = h.replace_status(id, &alice).await;
    assert_eq!(status, 400);
}

// ---------------------------------------------------------------------------
// Task 11: GET /api/me/stats
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn me_stats_counts_published_and_drafts_separately() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let p1 = h.seed_photo(&alice).await;
    h.wait_for_ready(p1).await;
    h.post_status(&format!("/api/photos/{p1}/publish"), None, Some(&alice))
        .await;
    let p2 = h.seed_photo(&alice).await;
    h.wait_for_ready(p2).await;
    h.post_status(&format!("/api/photos/{p2}/publish"), None, Some(&alice))
        .await;
    let draft = h.seed_photo(&alice).await;

    // Set exposure_s on photos to exercise integration_secs calculation
    h.put_status(
        &format!("/api/photos/{p1}"),
        &serde_json::json!({"exposure_s": 60.0}),
        Some(&alice),
    )
    .await;
    h.put_status(
        &format!("/api/photos/{draft}"),
        &serde_json::json!({"exposure_s": 30.0}),
        Some(&alice),
    )
    .await;

    let body = h.get_json("/api/me/stats", Some(&alice)).await;
    assert_eq!(body["published_count"], 2);
    assert_eq!(body["draft_count"], 1);
    assert_eq!(body["appreciations_received"], 0);
    // integration_secs should sum only published photos (60.0 from p1; nothing from draft)
    let integ = body["integration_secs"].as_f64().unwrap();
    assert!(
        (integ - 60.0).abs() < 0.001,
        "integration_secs should be 60.0, got {integ}"
    );
}

// ---------------------------------------------------------------------------
// Task 12: Hourly purge worker — drain stale photo_pending_deletes
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn purge_worker_sweeps_pending_deletes_older_than_7_days() {
    let (pool, _pg) = test_pool().await;
    let storage = std::sync::Arc::new(astrophoto::storage::MemoryStorage::new());
    let owner = Uuid::new_v4();
    let owner_handle = format!("o-{}", &owner.to_string()[..8]);
    sqlx::query!(
        "insert into users (id, email, password_hash, display_name, handle)
         values ($1, $2, '', 'O', $3)",
        owner,
        format!("o-{owner}@e"),
        owner_handle,
    )
    .execute(&pool)
    .await
    .unwrap();
    let id = sqlx::query_scalar!(
        "insert into photos (owner_id, storage_key, original_name, bytes, mime,
                             status, original_uploaded_at, last_step, short_id)
         values ($1, 'k', 'n.jpg', 10, 'image/jpeg', 'failed', now(), 'upload',
                 upper(left(replace(gen_random_uuid()::text, '-', ''), 8)))
         returning id",
        owner
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    storage
        .put("orphan-key", "image/jpeg", bytes::Bytes::from_static(b"x"))
        .await
        .unwrap();
    sqlx::query!(
        "insert into photo_pending_deletes (photo_id, storage_key, queued_at)
         values ($1, 'orphan-key', now() - interval '8 days')",
        id
    )
    .execute(&pool)
    .await
    .unwrap();

    // Fresh row — must NOT be swept (queued_at = now())
    storage
        .put("fresh-key", "image/jpeg", bytes::Bytes::from_static(b"y"))
        .await
        .unwrap();
    sqlx::query!(
        "insert into photo_pending_deletes (photo_id, storage_key, queued_at)
         values ($1, 'fresh-key', now())",
        id
    )
    .execute(&pool)
    .await
    .unwrap();

    astrophoto::jobs::purge_deletions::sweep_pending_deletes(&pool, storage.as_ref())
        .await
        .unwrap();

    let remaining: i64 = sqlx::query_scalar!(
        r#"select count(*) as "c!" from photo_pending_deletes where storage_key='orphan-key'"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(remaining, 0);
    assert!(
        storage.get("orphan-key").await.unwrap().is_none(),
        "S3 object swept"
    );

    let fresh_remaining: i64 = sqlx::query_scalar!(
        r#"select count(*) as "c!" from photo_pending_deletes where storage_key='fresh-key'"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(fresh_remaining, 1, "fresh row must survive sweep");
    assert!(
        storage.get("fresh-key").await.unwrap().is_some(),
        "fresh S3 object must survive"
    );
}

// ---------------------------------------------------------------------------
// Phase 8b final review fixes
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn delete_photo_204_for_owner_removes_row_and_s3() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let id = h.seed_photo(&alice).await;
    h.wait_for_ready(id).await;

    // Capture the master key for the post-delete S3 check.
    let key: String = sqlx::query_scalar!("select storage_key from photos where id=$1", id)
        .fetch_one(&h.pool)
        .await
        .unwrap();

    let status = h
        .delete_status(&format!("/api/photos/{id}"), Some(&alice))
        .await;
    assert_eq!(status, 204);

    let remaining: i64 =
        sqlx::query_scalar!(r#"select count(*) as "c!" from photos where id=$1"#, id)
            .fetch_one(&h.pool)
            .await
            .unwrap();
    assert_eq!(remaining, 0, "photos row must be gone after DELETE");

    // S3 master should be gone (MemoryStorage is in-process).
    assert!(
        h.storage.get(&key).await.unwrap().is_none(),
        "S3 master object must be swept by DELETE"
    );
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn delete_photo_403_for_non_owner() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let bob = h.signup("b@e.com", "longenoughpw", "Bob").await;
    let id = h.seed_photo(&alice).await;
    let status = h
        .delete_status(&format!("/api/photos/{id}"), Some(&bob))
        .await;
    assert_eq!(status, 403);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn pipeline_error_hidden_from_non_owner_and_anon() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let bob = h.signup("b@e.com", "longenoughpw", "Bob").await;
    let id = h.seed_photo(&alice).await;
    h.wait_for_ready(id).await;
    h.post_status(&format!("/api/photos/{id}/publish"), None, Some(&alice))
        .await;
    // Inject a fake pipeline_error directly into the DB.
    sqlx::query!(
        "update photos set status='failed', pipeline_error='internal: secret detail' where id=$1",
        id
    )
    .execute(&h.pool)
    .await
    .unwrap();

    let owner_view = h.get_json(&format!("/api/photos/{id}"), Some(&alice)).await;
    assert_eq!(
        owner_view["pipeline_error"].as_str(),
        Some("internal: secret detail"),
        "owner must see pipeline_error"
    );

    let anon_view = h.get_json(&format!("/api/photos/{id}"), None).await;
    assert!(
        anon_view["pipeline_error"].is_null(),
        "anonymous viewer must not see pipeline_error"
    );

    let bob_view = h.get_json(&format!("/api/photos/{id}"), Some(&bob)).await;
    assert!(
        bob_view["pipeline_error"].is_null(),
        "non-owner must not see pipeline_error"
    );
}
