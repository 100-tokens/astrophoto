//! Regression tests for the P0 prod-audit fixes around the photo
//! lifecycle: the finalize claim (5dbac04), the reaper/stuck-pipeline
//! sweeps (5dbac04), the pending-delete sweep guard + delete/cancel key
//! collection (6b68b96), and account-purge counter integrity (f6a775b).

#![allow(clippy::unwrap_used)]

mod common;

use std::sync::Arc;

use astrophoto::jobs::purge_deletions::{purge_once, sweep_pending_deletes};
use astrophoto::photos::cleanup::{reap_once, sweep_stuck_pipeline};
use astrophoto::photos::platesolve::{ABORTED_SENTINEL, SOLVING_SENTINEL};
use astrophoto::storage::{MemoryStorage, Storage};
use astrophoto::{db, http, mail::Mailer};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use bytes::Bytes;
use common::TestApp;
use testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner};
use testcontainers_modules::postgres::Postgres as PgImage;
use tower::ServiceExt;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Local harness: like `common::TestApp` but keeps a handle on the
// MemoryStorage the router uses, so tests can seed originals before a
// finalize and observe which S3 keys a delete/cancel actually removed.
// (common/mod.rs is owned by another change — helpers live here.)

struct StorageApp {
    app: axum::Router,
    pool: sqlx::PgPool,
    storage: Arc<MemoryStorage>,
    _pg: ContainerAsync<PgImage>,
}

async fn launch_with_storage() -> StorageApp {
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
    let (mailer, _outbox) = Mailer::for_test();
    let storage = Arc::new(MemoryStorage::new());
    let app = http::router(
        pool.clone(),
        common::config_for(&url),
        storage.clone(),
        Arc::new(mailer),
        None,
    );
    StorageApp {
        app,
        pool,
        storage,
        _pg: pg,
    }
}

/// Insert a photos row with the given status and storage key. Returns its id.
async fn insert_photo(
    pool: &sqlx::PgPool,
    owner_id: Uuid,
    storage_key: &str,
    mime: &str,
    status: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    let short_id = id.simple().to_string()[..8].to_string();
    sqlx::query!(
        r#"insert into photos
              (id, owner_id, storage_key, original_name, bytes, mime,
               status, short_id, last_step, original_uploaded_at)
           values ($1, $2, $3, 'test.jpg', 1000, $4, $5, $6, 'upload', now())"#,
        id,
        owner_id,
        storage_key,
        mime,
        status,
        short_id,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn post_finalize(
    app: &axum::Router,
    photo_id: Uuid,
    cookie: &str,
) -> (StatusCode, serde_json::Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/uploads/{photo_id}/finalize"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 1_048_576).await.unwrap();
    let v: serde_json::Value = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, v)
}

async fn send_delete(app: &axum::Router, uri: &str, cookie: &str) -> StatusCode {
    app.clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(uri)
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

async fn photo_status(pool: &sqlx::PgPool, id: Uuid) -> String {
    sqlx::query_scalar!("select status from photos where id = $1", id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn photo_count(pool: &sqlx::PgPool, id: Uuid) -> i64 {
    sqlx::query_scalar!(r#"select count(*) as "c!" from photos where id = $1"#, id)
        .fetch_one(pool)
        .await
        .unwrap()
}

// ---------------------------------------------------------------------------
// 1. Finalize claim (commit 5dbac04)

#[tokio::test]
async fn finalize_conflicts_while_another_finalize_holds_the_claim() {
    let t = launch_with_storage().await;
    let cookie = common::signup_and_cookie(&t.app, &t.pool, "claim@example.com", "claimuser").await;
    let user_id = common::lookup_user_id(&t.pool, "claim@example.com").await;

    let key = "originals/claim-conflict";
    let photo_id = insert_photo(&t.pool, user_id, key, "image/jpeg", "pending").await;
    t.storage
        .put(
            key,
            "image/jpeg",
            Bytes::from_static(include_bytes!("fixtures/sample.jpg")),
        )
        .await
        .unwrap();

    // Simulate a finalize mid-claim: the first call has flipped the row to
    // 'processing' and is running the pipeline. A duplicate finalize must
    // lose the claim and bounce with 409 instead of running the pipeline
    // a second time.
    sqlx::query!(
        "update photos set status = 'processing' where id = $1",
        photo_id
    )
    .execute(&t.pool)
    .await
    .unwrap();

    let (status, body) = post_finalize(&t.app, photo_id, &cookie).await;
    assert_eq!(status, 409, "duplicate finalize must conflict; got {body}");
    assert_eq!(body["error"], "conflict");
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("finalize already in progress"),
        "conflict message should say the finalize is already in progress; got {body}"
    );

    // The losing call must not have touched the row.
    assert_eq!(photo_status(&t.pool, photo_id).await, "processing");
}

#[tokio::test]
async fn finalize_reclaims_a_failed_row_and_succeeds() {
    let t = launch_with_storage().await;
    let cookie =
        common::signup_and_cookie(&t.app, &t.pool, "reclaim@example.com", "reclaimuser").await;
    let user_id = common::lookup_user_id(&t.pool, "reclaim@example.com").await;

    let key = "originals/claim-reclaim";
    let photo_id = insert_photo(&t.pool, user_id, key, "image/jpeg", "pending").await;
    t.storage
        .put(
            key,
            "image/jpeg",
            Bytes::from_static(include_bytes!("fixtures/sample.jpg")),
        )
        .await
        .unwrap();

    // A previous pipeline run died and marked the row failed. The claim
    // accepts 'failed' as a valid source state so a retry can recover.
    sqlx::query!(
        "update photos set status = 'failed', pipeline_error = 'boom' where id = $1",
        photo_id
    )
    .execute(&t.pool)
    .await
    .unwrap();

    let (status, body) = post_finalize(&t.app, photo_id, &cookie).await;
    assert_eq!(
        status, 200,
        "retry on a failed row must succeed; got {body}"
    );
    assert_eq!(body["status"], "ready");
    assert!(
        body["display_key"].is_string(),
        "display_key should be populated after the re-claimed finalize; got {body}"
    );

    let row = sqlx::query!(
        "select status, pipeline_error from photos where id = $1",
        photo_id
    )
    .fetch_one(&t.pool)
    .await
    .unwrap();
    assert_eq!(row.status, "ready");
    assert!(
        row.pipeline_error.is_none(),
        "pipeline_error must be cleared after a successful retry"
    );
}

// ---------------------------------------------------------------------------
// 2. Reaper safety: only old 'pending' rows are reaped.

#[tokio::test]
async fn reaper_never_deletes_processing_rows() {
    let app = TestApp::launch().await;
    let (_, user_id) = app
        .signup_with_handle("Reap", "reapuser", "reap@example.com")
        .await;

    let old_processing = insert_photo(
        &app.pool,
        user_id,
        "originals/p1",
        "image/jpeg",
        "processing",
    )
    .await;
    let old_pending =
        insert_photo(&app.pool, user_id, "originals/p2", "image/jpeg", "pending").await;
    let fresh_pending =
        insert_photo(&app.pool, user_id, "originals/p3", "image/jpeg", "pending").await;

    // Backdate both "old" rows past the 24h TTL; the fresh one stays young.
    sqlx::query!(
        "update photos set created_at = now() - interval '25 hours' where id = any($1)",
        &[old_processing, old_pending][..]
    )
    .execute(&app.pool)
    .await
    .unwrap();

    let storage = Arc::new(MemoryStorage::new());
    let reaped = reap_once(&app.pool, storage.as_ref()).await.unwrap();
    assert_eq!(reaped, 1, "only the old pending row may be reaped");

    assert_eq!(
        photo_count(&app.pool, old_processing).await,
        1,
        "a 25h-old 'processing' row must survive the reaper — an in-flight \
         finalize holds the claim and its assets must not be destroyed"
    );
    assert_eq!(photo_count(&app.pool, old_pending).await, 0);
    assert_eq!(photo_count(&app.pool, fresh_pending).await, 1);
}

// ---------------------------------------------------------------------------
// 3. sweep_stuck_pipeline (commit 5dbac04)

#[tokio::test]
async fn sweep_promotes_solved_awaiting_calibration_to_ready() {
    let app = TestApp::launch().await;
    let (_, user_id) = app
        .signup_with_handle("Sweep", "sweepuser", "sweep@example.com")
        .await;

    let solved = insert_photo(
        &app.pool,
        user_id,
        "originals/s1",
        "application/x-xisf",
        "awaiting-calibration",
    )
    .await;
    sqlx::query!(
        "update photos set platesolve_solved_at = now(), pipeline_error = 'stale' where id = $1",
        solved
    )
    .execute(&app.pool)
    .await
    .unwrap();

    sweep_stuck_pipeline(&app.pool).await.unwrap();

    let row = sqlx::query!(
        "select status, pipeline_error from photos where id = $1",
        solved
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(row.status, "ready", "solved row must be promoted to ready");
    assert!(row.pipeline_error.is_none());
}

#[tokio::test]
async fn sweep_fails_timed_out_calibration_and_swaps_solving_sentinel() {
    let app = TestApp::launch().await;
    let (_, user_id) = app
        .signup_with_handle("Sweep", "sweepuser", "sweep@example.com")
        .await;

    // (b) timed out: no solve landed and the calibration request is >30min old.
    let timed_out = insert_photo(
        &app.pool,
        user_id,
        "originals/s2",
        "application/x-xisf",
        "awaiting-calibration",
    )
    .await;
    sqlx::query!(
        "update photos
            set calibration_requested_at = now() - interval '31 minutes',
                platesolve_error = $2
          where id = $1",
        timed_out,
        SOLVING_SENTINEL
    )
    .execute(&app.pool)
    .await
    .unwrap();

    // Control: same shape but requested only 5 minutes ago — must be untouched.
    let in_window = insert_photo(
        &app.pool,
        user_id,
        "originals/s3",
        "application/x-xisf",
        "awaiting-calibration",
    )
    .await;
    sqlx::query!(
        "update photos
            set calibration_requested_at = now() - interval '5 minutes',
                platesolve_error = $2
          where id = $1",
        in_window,
        SOLVING_SENTINEL
    )
    .execute(&app.pool)
    .await
    .unwrap();

    sweep_stuck_pipeline(&app.pool).await.unwrap();

    let row = sqlx::query!(
        "select status, pipeline_error, platesolve_error from photos where id = $1",
        timed_out
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(row.status, "failed");
    assert!(
        row.pipeline_error
            .as_deref()
            .unwrap_or("")
            .contains("auto-calibration interrupted"),
        "pipeline_error should explain the interruption; got {:?}",
        row.pipeline_error
    );
    assert_eq!(
        row.platesolve_error.as_deref(),
        Some(ABORTED_SENTINEL),
        "a stale 'solving' sentinel must be swapped for the aborted sentinel \
         so a finalize retry doesn't silently no-op on AlreadySolving"
    );

    let control = sqlx::query!(
        "select status, platesolve_error from photos where id = $1",
        in_window
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(control.status, "awaiting-calibration");
    assert_eq!(control.platesolve_error.as_deref(), Some(SOLVING_SENTINEL));
}

#[tokio::test]
async fn sweep_fails_processing_rows_stuck_for_over_six_hours() {
    let app = TestApp::launch().await;
    let (_, user_id) = app
        .signup_with_handle("Sweep", "sweepuser", "sweep@example.com")
        .await;

    // (c) created long ago, never replaced: coalesce falls back to created_at.
    let stuck_created = insert_photo(
        &app.pool,
        user_id,
        "originals/s4",
        "image/jpeg",
        "processing",
    )
    .await;
    sqlx::query!(
        "update photos set created_at = now() - interval '7 hours' where id = $1",
        stuck_created
    )
    .execute(&app.pool)
    .await
    .unwrap();

    // (c') recent created_at but the replace that flipped it to processing
    // happened 7 hours ago: coalesce picks replaced_at.
    let stuck_replaced = insert_photo(
        &app.pool,
        user_id,
        "originals/s5",
        "image/jpeg",
        "processing",
    )
    .await;
    sqlx::query!(
        "update photos set replaced_at = now() - interval '7 hours' where id = $1",
        stuck_replaced
    )
    .execute(&app.pool)
    .await
    .unwrap();

    // Control: a live pipeline (fresh row) must not be touched.
    let live = insert_photo(
        &app.pool,
        user_id,
        "originals/s6",
        "image/jpeg",
        "processing",
    )
    .await;

    sweep_stuck_pipeline(&app.pool).await.unwrap();

    for id in [stuck_created, stuck_replaced] {
        let row = sqlx::query!(
            "select status, pipeline_error from photos where id = $1",
            id
        )
        .fetch_one(&app.pool)
        .await
        .unwrap();
        assert_eq!(row.status, "failed", "stuck processing row {id} must fail");
        assert!(
            row.pipeline_error
                .as_deref()
                .unwrap_or("")
                .contains("processing interrupted"),
            "pipeline_error should be the retryable interruption message"
        );
    }
    assert_eq!(photo_status(&app.pool, live).await, "processing");
}

// ---------------------------------------------------------------------------
// 4. sweep_pending_deletes guard (commit 6b68b96)

#[tokio::test]
async fn pending_delete_sweep_only_drains_rows_for_ready_photos() {
    let app = TestApp::launch().await;
    let (_, user_id) = app
        .signup_with_handle("Pend", "penduser", "pend@example.com")
        .await;

    // A replace that bricked: photo is 'failed' and the pending-delete rows
    // hold the PREVIOUS master/thumbs — the only good assets left.
    let photo = insert_photo(&app.pool, user_id, "originals/new", "image/jpeg", "failed").await;
    for key in ["originals/old-master", "thumbs/old-512.jpg"] {
        sqlx::query!(
            "insert into photo_pending_deletes (photo_id, storage_key, queued_at)
             values ($1, $2, now() - interval '8 days')",
            photo,
            key
        )
        .execute(&app.pool)
        .await
        .unwrap();
    }

    let storage = Arc::new(MemoryStorage::new());
    storage
        .put(
            "originals/old-master",
            "image/jpeg",
            Bytes::from_static(b"m"),
        )
        .await
        .unwrap();
    storage
        .put("thumbs/old-512.jpg", "image/jpeg", Bytes::from_static(b"t"))
        .await
        .unwrap();

    // Guard: photo not 'ready' → the sweep must not touch anything, no
    // matter how old the rows are.
    let swept = sweep_pending_deletes(&app.pool, storage.as_ref())
        .await
        .unwrap();
    assert_eq!(swept, 0, "sweep must skip rows whose photo is not ready");
    let remaining: i64 = sqlx::query_scalar!(
        r#"select count(*) as "c!" from photo_pending_deletes where photo_id = $1"#,
        photo
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(remaining, 2);
    assert!(
        storage.get("originals/old-master").await.unwrap().is_some(),
        "the archival master of a bricked photo must never be destroyed"
    );

    // Recovery: photo back to 'ready' → old rows drain. A freshly-queued row
    // (inside the 7-day window) must survive even then.
    sqlx::query!("update photos set status = 'ready' where id = $1", photo)
        .execute(&app.pool)
        .await
        .unwrap();
    sqlx::query!(
        "insert into photo_pending_deletes (photo_id, storage_key) values ($1, 'thumbs/fresh.jpg')",
        photo
    )
    .execute(&app.pool)
    .await
    .unwrap();

    let swept = sweep_pending_deletes(&app.pool, storage.as_ref())
        .await
        .unwrap();
    assert_eq!(swept, 2, "both stale rows drain once the photo is ready");
    let keys: Vec<String> = sqlx::query_scalar!(
        "select storage_key from photo_pending_deletes where photo_id = $1",
        photo
    )
    .fetch_all(&app.pool)
    .await
    .unwrap();
    assert_eq!(keys, vec!["thumbs/fresh.jpg".to_string()]);
    assert!(storage.get("originals/old-master").await.unwrap().is_none());
    assert!(storage.get("thumbs/old-512.jpg").await.unwrap().is_none());
}

// ---------------------------------------------------------------------------
// 5. Delete / cancel completeness (commit 6b68b96)

#[tokio::test]
async fn delete_photo_removes_every_storage_object_including_pending_deletes() {
    let t = launch_with_storage().await;
    let cookie = common::signup_and_cookie(&t.app, &t.pool, "del@example.com", "deluser").await;
    let user_id = common::lookup_user_id(&t.pool, "del@example.com").await;

    let master = "originals/del-master";
    let display = "display/del.jpg";
    let thumbs = ["thumbs/del-512.jpg", "thumbs/del-1024.jpg"];
    let pending = ["originals/del-prev-master", "thumbs/del-prev-512.jpg"];

    let photo = insert_photo(&t.pool, user_id, master, "image/jpeg", "ready").await;
    sqlx::query!(
        "update photos set display_key = $2 where id = $1",
        photo,
        display
    )
    .execute(&t.pool)
    .await
    .unwrap();
    for (i, key) in thumbs.iter().enumerate() {
        sqlx::query!(
            "insert into thumbnails (photo_id, size, storage_key, bytes) values ($1, $2, $3, 10)",
            photo,
            512 * (i as i32 + 1),
            *key
        )
        .execute(&t.pool)
        .await
        .unwrap();
    }
    // Keys stashed by an in-flight replace — CASCADE would silently drop the
    // rows, so the handler must fold the keys into the S3 batch first.
    for key in pending {
        sqlx::query!(
            "insert into photo_pending_deletes (photo_id, storage_key) values ($1, $2)",
            photo,
            key
        )
        .execute(&t.pool)
        .await
        .unwrap();
    }

    let all_keys: Vec<&str> = std::iter::once(master)
        .chain([display])
        .chain(thumbs)
        .chain(pending)
        .collect();
    for key in &all_keys {
        t.storage
            .put(key, "image/jpeg", Bytes::from_static(b"x"))
            .await
            .unwrap();
    }

    let status = send_delete(&t.app, &format!("/api/photos/{photo}"), &cookie).await;
    assert_eq!(status, 204);

    assert_eq!(photo_count(&t.pool, photo).await, 0);
    let orphan_rows: i64 = sqlx::query_scalar!(
        r#"select (select count(*) from thumbnails where photo_id = $1)
                 + (select count(*) from photo_pending_deletes where photo_id = $1) as "c!""#,
        photo
    )
    .fetch_one(&t.pool)
    .await
    .unwrap();
    assert_eq!(orphan_rows, 0, "CASCADE must remove thumbs + pending rows");

    for key in &all_keys {
        assert!(
            t.storage.get(key).await.unwrap().is_none(),
            "storage object {key} must be deleted — keys have to be collected \
             BEFORE the row delete or the CASCADE orphans them in S3"
        );
    }
}

#[tokio::test]
async fn cancel_upload_removes_master_display_and_thumbs() {
    let t = launch_with_storage().await;
    let cookie = common::signup_and_cookie(&t.app, &t.pool, "can@example.com", "canuser").await;
    let user_id = common::lookup_user_id(&t.pool, "can@example.com").await;

    // A cancel racing the pipeline: the photo is mid-'processing' and already
    // has a display master + a thumbnail in storage.
    let master = "originals/can-master";
    let display = "display/can.jpg";
    let thumb = "thumbs/can-512.jpg";
    let photo = insert_photo(&t.pool, user_id, master, "image/jpeg", "processing").await;
    sqlx::query!(
        "update photos set display_key = $2 where id = $1",
        photo,
        display
    )
    .execute(&t.pool)
    .await
    .unwrap();
    sqlx::query!(
        "insert into thumbnails (photo_id, size, storage_key, bytes) values ($1, 512, $2, 10)",
        photo,
        thumb
    )
    .execute(&t.pool)
    .await
    .unwrap();
    for key in [master, display, thumb] {
        t.storage
            .put(key, "image/jpeg", Bytes::from_static(b"x"))
            .await
            .unwrap();
    }

    let status = send_delete(&t.app, &format!("/api/uploads/{photo}"), &cookie).await;
    assert_eq!(status, 204);

    assert_eq!(photo_count(&t.pool, photo).await, 0);
    for key in [master, display, thumb] {
        assert!(
            t.storage.get(key).await.unwrap().is_none(),
            "cancel must delete {key} from storage"
        );
    }
}

#[tokio::test]
async fn cancel_refuses_published_photos() {
    let t = launch_with_storage().await;
    let cookie = common::signup_and_cookie(&t.app, &t.pool, "pub@example.com", "pubuser").await;
    let user_id = common::lookup_user_id(&t.pool, "pub@example.com").await;

    let photo = insert_photo(&t.pool, user_id, "originals/pub", "image/jpeg", "ready").await;
    sqlx::query!(
        "update photos set published_at = now() where id = $1",
        photo
    )
    .execute(&t.pool)
    .await
    .unwrap();

    let status = send_delete(&t.app, &format!("/api/uploads/{photo}"), &cookie).await;
    assert_eq!(status, 409, "a published photo is not cancellable");
    assert_eq!(photo_count(&t.pool, photo).await, 1);
}

// ---------------------------------------------------------------------------
// 6. Account purge integrity (commit f6a775b)

#[tokio::test]
async fn purging_a_user_decrements_appreciation_counters_on_others_photos() {
    let app = TestApp::launch().await;
    let (cookie_a, a_id) = app
        .signup_with_handle("Alice", "alicepurge", "alicepurge@example.com")
        .await;
    let (_, b_id) = app
        .signup_with_handle("Bob", "bobpurge", "bobpurge@example.com")
        .await;

    let photo = app.ready_photo(b_id).await;

    // A appreciates B's photo through the HTTP surface so the junction row
    // and the denormalized counter are written exactly as in prod.
    let (status, _) = app
        .oneshot(
            "POST",
            &format!("/api/photos/{photo}/appreciate"),
            Some(&cookie_a),
            None,
        )
        .await;
    assert_eq!(status, 204, "appreciate must succeed");
    let count: i32 = sqlx::query_scalar!(
        r#"select appreciations_count as "c!" from photos where id = $1"#,
        photo
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(count, 1);

    // Grace period elapsed → the hourly purge hard-deletes A.
    sqlx::query!(
        "update users set pending_deletion_at = now() - interval '1 hour' where id = $1",
        a_id
    )
    .execute(&app.pool)
    .await
    .unwrap();

    let storage = Arc::new(MemoryStorage::new());
    let purged = purge_once(&app.pool, storage.as_ref()).await.unwrap();
    assert_eq!(purged, 1, "exactly user A is due for purge");

    let a_left: i64 =
        sqlx::query_scalar!(r#"select count(*) as "c!" from users where id = $1"#, a_id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(a_left, 0, "user A must be hard-deleted");

    // B and B's photo survive, and the denormalized counter no longer
    // includes the purged user's appreciation (the CASCADE removes the
    // junction row; the purge must decrement the cached count too).
    let row = sqlx::query!(
        r#"select appreciations_count as "count!",
                  (select count(*) from appreciations where photo_id = $1) as "junction!"
             from photos where id = $1"#,
        photo
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(row.junction, 0, "junction row CASCADEs with the user");
    assert_eq!(
        row.count, 0,
        "appreciations_count must be decremented by the purge, not left stale"
    );
    let b_left: i64 =
        sqlx::query_scalar!(r#"select count(*) as "c!" from users where id = $1"#, b_id)
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(b_left, 1, "user B must be untouched");
}
