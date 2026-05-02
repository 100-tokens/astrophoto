//! Integration tests for Phase 8b: drafts, replace, my-photos stats,
//! visibility predicate. Phase 5 upload tests stay in `photos.rs`.

use std::sync::Arc;

use astrophoto::storage::MemoryStorage;
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
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;
use tower::ServiceExt;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Shared utilities (mirrored from photos.rs — not exposed as a crate item)
// ---------------------------------------------------------------------------

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
        oauth_google_client_id: String::new(),
        oauth_google_client_secret: String::new(),
        oauth_google_redirect_url: String::new(),
        smtp_host: "unused-in-tests".into(),
        smtp_port: 1025,
        smtp_user: String::new(),
        smtp_pass: String::new(),
        mail_from: "test <test@astrophoto.local>".into(),
        smtp_tls: false,
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
    // Kept for future tasks (5-11) that hit the DB layer directly.
    #[allow(dead_code)]
    pool: PgPool,
    _pg: testcontainers::ContainerAsync<PgImage>,
}

/// Spin up a fresh Postgres container, run migrations, and build the axum
/// router with MemoryStorage and a test mailer.
#[allow(clippy::unwrap_used)]
async fn harness() -> H {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let storage = Arc::new(MemoryStorage::new());
    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    let app = http::router(pool.clone(), config_for(&url), storage, Arc::new(mailer));

    H { app, pool, _pg: pg }
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

    /// POST /api/auth/signup, returns the `set-cookie` header value.
    #[allow(clippy::unwrap_used)]
    async fn signup(&self, email: &str, password: &str, display_name: &str) -> String {
        let body = serde_json::json!({
            "email": email,
            "password": password,
            "display_name": display_name,
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
        assert_eq!(resp.status(), 201, "signup failed for {email}");
        resp.headers()
            .get("set-cookie")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    }

    /// POST a minimal JPEG multipart to /api/photos (draft upload).
    /// Returns the new photo id.
    #[allow(clippy::unwrap_used)]
    async fn upload_draft(&self, cookie: &str) -> Uuid {
        let boundary = "----phase8btestboundary";
        let jpeg = make_test_jpeg();
        let mut body: Vec<u8> = Vec::new();
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            b"Content-Disposition: form-data; name=\"file\"; filename=\"draft.jpg\"\r\n",
        );
        body.extend_from_slice(b"Content-Type: image/jpeg\r\n\r\n");
        body.extend_from_slice(&jpeg);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

        let resp = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/photos")
                    .header(header::COOKIE, cookie)
                    .header(
                        header::CONTENT_TYPE,
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 202, "upload_draft returned non-202");
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        Uuid::parse_str(v["id"].as_str().unwrap()).unwrap()
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
    let pg = PgImage::default().start().await.unwrap();
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
    sqlx::query!(
        "insert into users (id, email, password_hash, display_name)
         values ($1, $2, '', 'O'), ($3, $4, '', 'V')",
        owner,
        format!("o-{owner}@e"),
        viewer,
        format!("v-{viewer}@e")
    )
    .execute(&pool)
    .await
    .unwrap();
    let photo_id = sqlx::query_scalar!(
        "insert into photos (owner_id, storage_key, original_name, bytes, mime,
                             status, published_at, original_uploaded_at, last_step)
         values ($1, 'k', 'n.jpg', 10, 'image/jpeg', 'ready', now(), now(), 'caption')
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
    sqlx::query!(
        "insert into users (id, email, password_hash, display_name)
         values ($1, $2, '', 'O'), ($3, $4, '', 'V')",
        owner,
        format!("o-{owner}@e"),
        viewer,
        format!("v-{viewer}@e")
    )
    .execute(&pool)
    .await
    .unwrap();
    let photo_id = sqlx::query_scalar!(
        "insert into photos (owner_id, storage_key, original_name, bytes, mime,
                             status, original_uploaded_at, last_step)
         values ($1, 'k', 'n.jpg', 10, 'image/jpeg', 'processing', now(), 'upload')
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
    sqlx::query!(
        "insert into users (id, email, password_hash, display_name)
         values ($1, $2, '', 'O')",
        owner,
        format!("o-{owner}@e")
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

    let alice_draft_id = h.upload_draft(&alice).await;
    h.upload_draft(&bob).await; // draft for bob

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
    let photo_id = h.upload_draft(&alice).await;

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
    let photo_id = h.upload_draft(&alice).await;

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
    let photo_id = h.upload_draft(&alice).await;

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
    let photo_id = h.upload_draft(&alice).await;

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
    let photo_id = h.upload_draft(&alice).await;

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
    let id = h.upload_draft(&alice).await;
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
    let id = h.upload_draft(&alice).await;
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
    let id = h.upload_draft(&alice).await;
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
    // Insert directly (bypassing upload_draft) so no background pipeline races.
    let id = sqlx::query_scalar!(
        "insert into photos (owner_id, storage_key, original_name, bytes, mime,
                             status, last_step, original_uploaded_at)
         values ($1, 'k', 'n.jpg', 10, 'image/jpeg', 'processing', 'upload', now())
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
    let draft = h.upload_draft(&alice).await;
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
    let id = h.upload_draft(&alice).await;
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
    let id = h.upload_draft(&alice).await;
    h.wait_for_ready(id).await;

    // Set target to a value
    let body = serde_json::json!({ "target": "M31" });
    h.put_status(&format!("/api/photos/{id}"), &body, Some(&alice)).await;
    let row = sqlx::query!("select target from photos where id=$1", id)
        .fetch_one(&h.pool).await.unwrap();
    assert_eq!(row.target.as_deref(), Some("M31"));

    // Now explicitly clear it via JSON null
    let body = serde_json::json!({ "target": null });
    let status = h.put_status(&format!("/api/photos/{id}"), &body, Some(&alice)).await;
    assert_eq!(status, 200);
    let row = sqlx::query!("select target from photos where id=$1", id)
        .fetch_one(&h.pool).await.unwrap();
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
    sqlx::query!(
        "insert into users (id, email, password_hash, display_name)
         values ($1, $2, '', 'O')", owner, format!("o-{owner}@e")
    ).execute(&pool).await.unwrap();
    let id = sqlx::query_scalar!(
        "insert into photos (owner_id, storage_key, original_name, bytes, mime,
                             status, original_uploaded_at, last_step)
         values ($1, 'k', 'n.jpg', 10, 'image/jpeg', 'processing', now(), 'upload')
         returning id", owner
    ).fetch_one(&pool).await.unwrap();

    astrophoto::photos::queries::mark_failed(&pool, id, "decode failed: bad jpeg").await.unwrap();
    let row = sqlx::query!("select status, pipeline_error from photos where id=$1", id)
        .fetch_one(&pool).await.unwrap();
    assert_eq!(row.status, "failed");
    assert_eq!(row.pipeline_error.as_deref(), Some("decode failed: bad jpeg"));
}

// ---------------------------------------------------------------------------
// Task 10: Replace endpoint
// ---------------------------------------------------------------------------

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn replace_swaps_storage_key_keeps_metadata() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let id = h.upload_draft(&alice).await;
    h.wait_for_ready(id).await;
    h.put_status(
        &format!("/api/photos/{id}"),
        &serde_json::json!({"target":"M31","caption":"v1"}),
        Some(&alice),
    )
    .await;
    h.post_status(&format!("/api/photos/{id}/publish"), None, Some(&alice))
        .await;

    let key_before: String =
        sqlx::query_scalar!("select storage_key from photos where id=$1", id)
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
        r#"select count(*) as "c!" from photo_pending_deletes where photo_id = $1"#, id
    ).fetch_one(&h.pool).await.unwrap();
    assert_eq!(pending, 0, "pending deletes must be drained after replace");

    let thumb_count: i64 = sqlx::query_scalar!(
        r#"select count(*) as "c!" from thumbnails where photo_id = $1"#, id
    ).fetch_one(&h.pool).await.unwrap();
    assert!(thumb_count > 0, "thumbnails must be regenerated after replace");
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn replace_403_for_non_owner() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let bob = h.signup("b@e.com", "longenoughpw", "Bob").await;
    let id = h.upload_draft(&alice).await;
    h.wait_for_ready(id).await;
    let status = h.replace_status(id, &bob).await;
    assert_eq!(status, 403);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn replace_400_when_pipeline_busy() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let id = h.upload_draft(&alice).await;
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
    let p1 = h.upload_draft(&alice).await;
    h.wait_for_ready(p1).await;
    h.post_status(&format!("/api/photos/{p1}/publish"), None, Some(&alice))
        .await;
    let p2 = h.upload_draft(&alice).await;
    h.wait_for_ready(p2).await;
    h.post_status(&format!("/api/photos/{p2}/publish"), None, Some(&alice))
        .await;
    let _draft = h.upload_draft(&alice).await;

    let body = h.get_json("/api/me/stats", Some(&alice)).await;
    assert_eq!(body["published_count"], 2);
    assert_eq!(body["draft_count"], 1);
    assert_eq!(body["appreciations_received"], 0);
}
