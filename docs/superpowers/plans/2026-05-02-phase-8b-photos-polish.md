# Phase 8b — Photos, Drafts, Replace & Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the My Photos page, drafts surface, replace-image flow, and Polish 8.5 micro-fixes from the Phase 8 design that Phase 8a deferred.

**Architecture:** A single migration adds `published_at`, `replaced_at`, `original_uploaded_at`, `last_step`, `pipeline_error`, and a `photo_pending_deletes` table. Backend gains a single `is_visible_to` predicate used by every public per-photo endpoint, plus `publish`, `metadata` (PUT), `replace`, and `me::stats` modules. The pipeline gains a `pipeline_options` enum that toggles "skip user-edited EXIF" on replace and a "drain pending S3 deletes on success" branch. The frontend splits upload into 3 server-side routes (`/upload`, `/upload/[id]/verify`, `/upload/[id]/caption`), adds `/account/frames` as the per-owner dashboard, and ships seven new components plus the four Polish 8.5 micro-fixes.

**Tech Stack:** Rust 2024 + axum 0.7 + sqlx 0.8 (compile-time checked); SvelteKit 2 + Svelte 5 runes + ts-rs (Rust→TS); Postgres 16; aws-sdk-s3 1 (Cloudflare R2 in prod, MinIO in dev); Playwright for e2e.

**Spec:** `docs/superpowers/specs/2026-05-02-phase-8b-photos-polish-design.md` — read before starting.

---

## Branch and worktree

Before Task 1, create the worktree (handled by subagent-driven-development):

```bash
git worktree add ../astrophoto-phase-8b -b feat/phase-8b-photos main
cd ../astrophoto-phase-8b
```

All commits go onto `feat/phase-8b-photos`. Merge via `superpowers:finishing-a-development-branch` after Task 23.

---

## Task 1: Migration 0004 — drafts, replace, pipeline error, pending deletes

**Files:**
- Create: `backend/migrations/0004_drafts_replace.sql`

- [ ] **Step 1: Create the migration file**

```sql
-- Phase 8b: drafts, replace tracking, pipeline error capture,
-- and a deferred-S3-delete table used by the replace endpoint to
-- avoid the data-loss window where a corrupt new master would leave
-- the user with no recoverable original.

-- 1. Drafts: published_at NULL = draft, NOT NULL = published.
--    Pipeline state (status) and publish state stay separate concerns.
alter table photos
  add column published_at timestamptz;

create index photos_published_at_idx on photos (published_at desc)
  where published_at is not null;

create index photos_drafts_owner_idx on photos (owner_id, created_at desc)
  where published_at is null;

-- Backfill: every existing 'ready' photo is considered published at its
-- creation time. 'processing' / 'failed' rows stay draft (NULL).
update photos set published_at = created_at where status = 'ready';

-- 2. Replace tracking.
alter table photos
  add column replaced_at timestamptz,
  add column original_uploaded_at timestamptz;

update photos set original_uploaded_at = created_at;
alter table photos alter column original_uploaded_at set not null;

-- 3. Track upload progress for the draft card chrome.
alter table photos
  add column last_step text
    check (last_step in ('upload', 'verify', 'caption'));

update photos set last_step = 'caption'
  where status = 'ready' and published_at is not null;
update photos set last_step = 'upload'
  where status in ('processing', 'failed');

-- 4. Pipeline error capture: written when the pipeline marks a row
--    'failed' so the verify-step UI can surface the reason and the
--    user can choose Discard vs Retry.
alter table photos
  add column pipeline_error text;

-- 5. Deferred S3 deletion table — populated by the replace endpoint,
--    drained by the pipeline on successful 'ready' transition or by
--    the hourly purge worker for rows older than 7 days.
create table photo_pending_deletes (
  id          bigserial primary key,
  photo_id    uuid not null references photos(id) on delete cascade,
  storage_key text not null,
  queued_at   timestamptz not null default now()
);

create index photo_pending_deletes_photo_idx
  on photo_pending_deletes (photo_id);

create index photo_pending_deletes_queued_idx
  on photo_pending_deletes (queued_at);
```

- [ ] **Step 2: Run the migration locally and verify it applies cleanly**

Run: `just db-reset`
Expected: no errors; `psql $DATABASE_URL -c "\d photos"` shows the new columns; `\d photo_pending_deletes` shows the new table.

- [ ] **Step 3: Commit**

```bash
git add backend/migrations/0004_drafts_replace.sql
git commit -m "feat(db): migration 0004 — drafts, replace tracking, pending deletes"
```

---

## Task 2: Extend `PhotoRow` and add the `is_visible_to` visibility predicate

**Files:**
- Modify: `backend/src/photos/queries.rs`
- Test: `backend/tests/photos_phase8b.rs` (new)

The single-source-of-truth predicate that every public per-photo endpoint must call. Encodes `published_at IS NOT NULL OR owner_id = viewer_id`, so future endpoints can't accidentally skip the check.

- [ ] **Step 1: Write the failing test**

Create `backend/tests/photos_phase8b.rs` with the test harness boilerplate copied verbatim from `backend/tests/photos.rs:1-95` (config_for, make_test_jpeg, signup helper). Add a fresh module docstring at the top:

```rust
//! Integration tests for Phase 8b: drafts, replace, my-photos stats,
//! visibility predicate. Phase 5 upload tests stay in `photos.rs`.
```

Then append:

```rust
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn is_visible_to_returns_true_for_published_to_anyone() {
    let (pool, _pg) = test_pool().await;
    let owner = Uuid::new_v4();
    let viewer = Uuid::new_v4();
    sqlx::query!(
        "insert into users (id, email, password_hash, display_name)
         values ($1, $2, '', 'O'), ($3, $4, '', 'V')",
        owner, format!("o-{owner}@e"), viewer, format!("v-{viewer}@e")
    ).execute(&pool).await.unwrap();
    let photo_id = sqlx::query_scalar!(
        "insert into photos (owner_id, storage_key, original_name, bytes, mime,
                             status, published_at, original_uploaded_at, last_step)
         values ($1, 'k', 'n.jpg', 10, 'image/jpeg', 'ready', now(), now(), 'caption')
         returning id",
        owner
    ).fetch_one(&pool).await.unwrap();

    assert!(astrophoto::photos::queries::is_visible_to(&pool, photo_id, Some(viewer)).await.unwrap());
    assert!(astrophoto::photos::queries::is_visible_to(&pool, photo_id, None).await.unwrap());
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
        owner, format!("o-{owner}@e"), viewer, format!("v-{viewer}@e")
    ).execute(&pool).await.unwrap();
    let photo_id = sqlx::query_scalar!(
        "insert into photos (owner_id, storage_key, original_name, bytes, mime,
                             status, original_uploaded_at, last_step)
         values ($1, 'k', 'n.jpg', 10, 'image/jpeg', 'processing', now(), 'upload')
         returning id",
        owner
    ).fetch_one(&pool).await.unwrap();

    assert!(astrophoto::photos::queries::is_visible_to(&pool, photo_id, Some(viewer)).await.unwrap() == false);
    assert!(astrophoto::photos::queries::is_visible_to(&pool, photo_id, None).await.unwrap() == false);
    assert!(astrophoto::photos::queries::is_visible_to(&pool, photo_id, Some(owner)).await.unwrap());
}
```

Add a `test_pool()` helper near the top:

```rust
async fn test_pool() -> (sqlx::PgPool, testcontainers::ContainerAsync<PgImage>) {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    (pool, pg)
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd backend && cargo test --test photos_phase8b is_visible_to -- --nocapture`
Expected: FAIL — "no function `is_visible_to`".

- [ ] **Step 3: Add the helper**

Append to `backend/src/photos/queries.rs`:

```rust
/// Returns true if `viewer_id` may see `photo_id` on a public surface.
/// Encodes the visibility rule once: a photo is visible if it's published
/// (`published_at IS NOT NULL`) OR if the viewer owns it. Used by every
/// public per-photo endpoint (detail, counts, comments list) so the
/// predicate lives in one place and future endpoints inherit it.
pub async fn is_visible_to(
    pool: &PgPool,
    photo_id: Uuid,
    viewer_id: Option<Uuid>,
) -> Result<bool, AppError> {
    let row = sqlx::query!(
        r#"select published_at, owner_id from photos where id = $1"#,
        photo_id
    )
    .fetch_optional(pool)
    .await?;
    Ok(match (row, viewer_id) {
        (None, _) => false,
        (Some(r), _) if r.published_at.is_some() => true,
        (Some(r), Some(v)) if r.owner_id == v => true,
        _ => false,
    })
}
```

Also extend `PhotoRow` in the same file:

```rust
pub struct PhotoRow {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub storage_key: String,
    pub original_name: String,
    pub bytes: i64,
    pub mime: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub taken_at: Option<DateTime<Utc>>,
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub iso: Option<i32>,
    pub exposure_s: Option<f64>,
    pub focal_mm: Option<f64>,
    pub target: Option<String>,
    pub caption: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub replaced_at: Option<DateTime<Utc>>,
    pub original_uploaded_at: DateTime<Utc>,
    pub last_step: Option<String>,
    pub pipeline_error: Option<String>,
}
```

- [ ] **Step 4: Run sqlx prepare and tests**

```bash
cd backend
cargo sqlx prepare
cargo test --test photos_phase8b is_visible_to -- --nocapture
```
Expected: PASS for both new tests; existing `cargo test` still green.

- [ ] **Step 5: Commit**

```bash
git add backend/src/photos/queries.rs backend/tests/photos_phase8b.rs backend/.sqlx/
git commit -m "feat(photos): is_visible_to predicate + extend PhotoRow with draft/replace columns"
```

---

## Task 3: Update `PhotoRow` consumers + extend insert/list/find queries

**Files:**
- Modify: `backend/src/photos/queries.rs` — every `select` that returns `PhotoRow` must include the new columns.
- Modify: `backend/src/photos/queries.rs::insert_processing` — set `last_step = 'upload'`, `original_uploaded_at = now()`. (Backfill of existing rows is handled by Migration 0004.)
- Test: append to `backend/tests/photos_phase8b.rs`.

- [ ] **Step 1: Write the failing test**

Append to `backend/tests/photos_phase8b.rs`:

```rust
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn insert_processing_sets_last_step_upload_and_published_at_null() {
    let (pool, _pg) = test_pool().await;
    let owner = Uuid::new_v4();
    sqlx::query!(
        "insert into users (id, email, password_hash, display_name)
         values ($1, $2, '', 'O')",
        owner, format!("o-{owner}@e")
    ).execute(&pool).await.unwrap();

    let photo_id = astrophoto::photos::queries::insert_processing(
        &pool, owner, "k", "n.jpg", 10, "image/jpeg", None, None
    ).await.unwrap();

    let row = sqlx::query!(
        "select published_at, last_step, original_uploaded_at from photos where id = $1",
        photo_id
    ).fetch_one(&pool).await.unwrap();

    assert!(row.published_at.is_none());
    assert_eq!(row.last_step.as_deref(), Some("upload"));
    assert!(row.original_uploaded_at <= chrono::Utc::now());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd backend && cargo test --test photos_phase8b insert_processing_sets_last_step_upload -- --nocapture`
Expected: FAIL — `last_step` is NULL because the existing INSERT doesn't set it.

- [ ] **Step 3: Update every query that touches `PhotoRow`**

Replace the body of `insert_processing` in `backend/src/photos/queries.rs`:

```rust
#[allow(clippy::too_many_arguments)]
pub async fn insert_processing(
    pool: &PgPool,
    owner_id: Uuid,
    storage_key: &str,
    original_name: &str,
    bytes: i64,
    mime: &str,
    target: Option<&str>,
    caption: Option<&str>,
) -> Result<Uuid, AppError> {
    let row = sqlx::query!(
        r#"
        insert into photos (owner_id, storage_key, original_name, bytes, mime,
                            target, caption, status, last_step, original_uploaded_at)
        values ($1, $2, $3, $4, $5, $6, $7, 'processing', 'upload', now())
        returning id
        "#,
        owner_id, storage_key, original_name, bytes, mime, target, caption,
    )
    .fetch_one(pool)
    .await?;
    Ok(row.id)
}
```

Replace `find_by_id` to select all `PhotoRow` columns:

```rust
pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<PhotoRow>, AppError> {
    let row = sqlx::query_as!(
        PhotoRow,
        r#"
        select id, owner_id, storage_key, original_name, bytes, mime,
               width, height, taken_at, camera, lens, iso, exposure_s, focal_mm,
               target, caption, status, created_at,
               published_at, replaced_at, original_uploaded_at, last_step, pipeline_error
        from photos where id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
```

Apply the same column expansion to `list_by_owner`, `list_recent_public`, `list_following`. Each one MUST add `published_at, replaced_at, original_uploaded_at, last_step, pipeline_error` to its `select` and to the qualifying `p.` prefix in `list_following`.

For `list_recent_public` and `list_following`, change the WHERE to gate on `published_at IS NOT NULL` instead of `status = 'ready'` (a published photo can briefly be `processing` after a replace — gallery should keep showing it; the binary served is the new one in S3, and the thumb endpoint already handles the missing-thumb case as 404 which the frontend treats as the placeholder):

```rust
// list_recent_public
where published_at is not null
order by published_at desc

// list_following
where f.follower_id = $1 and p.published_at is not null
order by p.published_at desc
```

For `list_by_owner`, drop the `status = 'ready'` clause entirely (this powers the public profile feed; gate on `published_at IS NOT NULL`):

```rust
where owner_id = $1 and published_at is not null
order by published_at desc
```

For `count_by_owner`, switch to `published_at IS NOT NULL`:

```rust
"select count(*) as \"count!\" from photos where owner_id = $1 and published_at is not null"
```

- [ ] **Step 4: Run sqlx prepare and the full test**

```bash
cd backend
cargo sqlx prepare
cargo test --test photos_phase8b -- --nocapture
cargo test --test photos -- --nocapture   # the Phase 5 upload test must still pass
```
Expected: all tests pass. The Phase 5 test polls until `status = 'ready'` and asserts thumbnails exist — both still hold because `mark_ready` hasn't been touched yet.

- [ ] **Step 5: Commit**

```bash
git add backend/src/photos/queries.rs backend/tests/photos_phase8b.rs backend/.sqlx/
git commit -m "feat(photos): insert sets last_step+original_uploaded_at; visibility filters use published_at"
```

---

## Task 4: List endpoint — drafts query support

**Files:**
- Modify: `backend/src/photos/queries.rs` — add `list_drafts_by_owner`.
- Modify: `backend/src/photos/list.rs` — handle `?drafts=true`; reject cross-user `?owner_id=&drafts=true`.
- Test: append to `backend/tests/photos_phase8b.rs`.

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn list_drafts_returns_only_callers_drafts() {
    let h = harness().await;
    let alice = h.signup("alice@e.com", "longenoughpw", "Alice").await;
    let bob = h.signup("bob@e.com", "longenoughpw", "Bob").await;

    h.upload_draft(&alice).await;        // draft for alice
    h.upload_published(&alice).await;    // published for alice
    h.upload_draft(&bob).await;          // draft for bob

    let body = h.get_json("/api/photos?drafts=true", Some(&alice)).await;
    let photos = body["photos"].as_array().unwrap();
    assert_eq!(photos.len(), 1, "alice sees only her own draft");
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn list_drafts_with_cross_user_owner_id_is_rejected() {
    let h = harness().await;
    let alice = h.signup("alice@e.com", "longenoughpw", "Alice").await;
    let bob = h.signup("bob@e.com", "longenoughpw", "Bob").await;
    let bob_id = h.user_id(&bob).await;

    let status = h.get_status(
        &format!("/api/photos?drafts=true&owner_id={bob_id}"),
        Some(&alice)
    ).await;
    assert_eq!(status, 403);
}
```

The harness functions (`harness`, `signup`, `upload_draft`, `upload_published`, `get_json`, `get_status`, `user_id`) need to be added — extract the boilerplate from `backend/tests/photos.rs` into a `mod common` at the top of `photos_phase8b.rs`:

```rust
mod common {
    use super::*;
    use astrophoto::storage::MemoryStorage;
    use astrophoto::{Config, db, http};
    use axum::body::Body;
    use axum::http::{Request, header};
    use http_body_util::BodyExt as _;
    use std::sync::Arc;
    use tower::ServiceExt;

    pub struct H {
        pub app: axum::Router,
        pub pool: sqlx::PgPool,
    }

    pub async fn harness() -> H {
        let (pool, _pg) = super::test_pool().await;
        let storage = Arc::new(MemoryStorage::new());
        let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
        let url = "unused-after-pool";
        let app = http::router(pool.clone(), super::config_for(url), storage, Arc::new(mailer));
        // Leak the pg container by stashing it in a thread-local? simpler:
        // hold _pg in the H struct as well — reorganise:
        H { app, pool }
    }
    // … signup / upload_draft / upload_published / get_json / get_status / user_id
}
```

(The harness keeps the testcontainer alive via the pool; for new tests the pattern from `photos.rs:65-180` should be copy-adapted into the helpers. Implementer: use the existing photos.rs flow as the reference — sign up via `POST /api/auth/signup`, capture the `set-cookie`, upload via `POST /api/photos` multipart, and for drafts post a `?status_after=draft` workaround? — **No**: draft is the new default, so any `POST /api/photos` produces a draft. Publication happens via `POST /api/photos/:id/publish`, added in Task 7. So in this task, `upload_draft` is just "upload"; `upload_published` calls upload + publish. Defer `upload_published` until after Task 7 — for THIS task, only `upload_draft` is needed.)

Adjust the test plan: in Task 4, only test `list_drafts_returns_only_callers_drafts` and the 403 case. Both can be set up by uploading two drafts (one per user) and asserting alice sees only one.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd backend && cargo test --test photos_phase8b list_drafts -- --nocapture`
Expected: FAIL — handler doesn't recognise `drafts=true` and the SQL doesn't filter on it.

- [ ] **Step 3: Add `list_drafts_by_owner`**

Append to `backend/src/photos/queries.rs`:

```rust
pub async fn list_drafts_by_owner(
    pool: &PgPool,
    owner_id: Uuid,
    limit: i64,
) -> Result<Vec<PhotoRow>, AppError> {
    let rows = sqlx::query_as!(
        PhotoRow,
        r#"
        select id, owner_id, storage_key, original_name, bytes, mime,
               width, height, taken_at, camera, lens, iso, exposure_s, focal_mm,
               target, caption, status, created_at,
               published_at, replaced_at, original_uploaded_at, last_step, pipeline_error
        from photos
        where owner_id = $1 and published_at is null
        order by created_at desc
        limit $2
        "#,
        owner_id, limit
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
```

- [ ] **Step 4: Wire the handler**

Replace `backend/src/photos/list.rs::handler`:

```rust
use crate::auth::middleware::OptionalUser;

#[derive(Deserialize)]
pub struct ListQuery {
    pub owner_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub following: Option<bool>,
    pub drafts: Option<bool>,
}

pub async fn handler(
    State(state): State<AppState>,
    user: OptionalUser,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListResponse>, AppError> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);

    if q.drafts.unwrap_or(false) {
        let me = user.0.ok_or(AppError::Unauthorized)?;
        // Reject cross-user drafts — users can only ever see their own.
        if let Some(requested) = q.owner_id {
            if requested != me.id {
                return Err(AppError::Forbidden);
            }
        }
        let rows = queries::list_drafts_by_owner(&state.pool, me.id, limit).await?;
        return Ok(Json(ListResponse {
            photos: rows.into_iter().map(Into::into).collect(),
        }));
    }

    let rows = if q.following.unwrap_or(false) {
        let follower = user.0.ok_or(AppError::Unauthorized)?;
        queries::list_following(&state.pool, follower.id, limit).await?
    } else if let Some(id) = q.owner_id {
        queries::list_by_owner(&state.pool, id, limit).await?
    } else {
        queries::list_recent_public(&state.pool, limit).await?
    };

    Ok(Json(ListResponse {
        photos: rows.into_iter().map(Into::into).collect(),
    }))
}
```

(`AppError::Forbidden` is a unit variant in `backend/src/error.rs`; it maps to 403. The handler returns the plain variant — no payload.)

- [ ] **Step 5: Run tests**

```bash
cd backend
cargo sqlx prepare
cargo test --test photos_phase8b -- --nocapture
```
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add backend/src/photos/queries.rs backend/src/photos/list.rs backend/tests/photos_phase8b.rs backend/.sqlx/
git commit -m "feat(photos): GET /api/photos?drafts=true (owner-only, cross-user 403)"
```

---

## Task 5: Photo detail — visibility 404 + extended DTO

**Files:**
- Modify: `backend/src/photos/get.rs`
- Test: append to `backend/tests/photos_phase8b.rs`.

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn get_draft_returns_404_for_non_owner() {
    let h = harness().await;
    let alice = h.signup("alice@e.com", "longenoughpw", "Alice").await;
    let bob = h.signup("bob@e.com", "longenoughpw", "Bob").await;
    let photo_id = h.upload_draft(&alice).await;

    let status = h.get_status(&format!("/api/photos/{photo_id}"), Some(&bob)).await;
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

    let body = h.get_json(&format!("/api/photos/{photo_id}"), Some(&alice)).await;
    assert_eq!(body["is_draft"], true);
    assert!(body["last_step"].as_str().is_some());
    assert!(body["replaced_at"].is_null());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd backend && cargo test --test photos_phase8b get_draft -- --nocapture`
Expected: FAIL — both: 200 returned for non-owner, and `is_draft` field missing.

- [ ] **Step 3: Update `get.rs`**

Replace `backend/src/photos/get.rs` body in full:

```rust
use axum::{
    Json,
    extract::{Path, State},
};
use serde::Serialize;
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::OptionalUser;
use crate::http::AppState;
use crate::photos::queries::{self, PhotoRow};

#[derive(Serialize)]
pub struct PhotoDetail {
    pub id: String,
    pub owner_id: String,
    pub status: String,
    pub original_name: String,
    pub bytes: i64,
    pub mime: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub iso: Option<i32>,
    pub exposure_s: Option<f64>,
    pub focal_mm: Option<f64>,
    pub target: Option<String>,
    pub caption: Option<String>,
    pub taken_at: Option<String>,
    pub created_at: String,
    pub appreciation_count: i64,
    pub comment_count: i64,
    pub is_draft: bool,
    pub last_step: Option<String>,
    pub replaced_at: Option<String>,
    pub original_uploaded_at: String,
    pub pipeline_error: Option<String>,
}

impl From<PhotoRow> for PhotoDetail {
    fn from(p: PhotoRow) -> Self {
        Self {
            id: p.id.to_string(),
            owner_id: p.owner_id.to_string(),
            status: p.status,
            original_name: p.original_name,
            bytes: p.bytes,
            mime: p.mime,
            width: p.width,
            height: p.height,
            camera: p.camera,
            lens: p.lens,
            iso: p.iso,
            exposure_s: p.exposure_s,
            focal_mm: p.focal_mm,
            target: p.target,
            caption: p.caption,
            taken_at: p.taken_at.map(|d| d.to_rfc3339()),
            created_at: p.created_at.to_rfc3339(),
            appreciation_count: 0,
            comment_count: 0,
            is_draft: p.published_at.is_none(),
            last_step: p.last_step,
            replaced_at: p.replaced_at.map(|d| d.to_rfc3339()),
            original_uploaded_at: p.original_uploaded_at.to_rfc3339(),
            pipeline_error: p.pipeline_error,
        }
    }
}

pub async fn handler(
    State(state): State<AppState>,
    user: OptionalUser,
    Path(id): Path<Uuid>,
) -> Result<Json<PhotoDetail>, AppError> {
    let viewer = user.0.as_ref().map(|u| u.id);
    if !queries::is_visible_to(&state.pool, id, viewer).await? {
        return Err(AppError::not_found("photo"));
    }

    let row = queries::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::not_found("photo"))?;

    let appreciation_count = sqlx::query!(
        r#"select count(*) as "count!" from appreciations where photo_id = $1"#,
        id
    )
    .fetch_one(&state.pool)
    .await?
    .count;

    let comment_count = sqlx::query!(
        r#"select count(*) as "count!" from comments where photo_id = $1"#,
        id
    )
    .fetch_one(&state.pool)
    .await?
    .count;

    let mut dto: PhotoDetail = row.into();
    dto.appreciation_count = appreciation_count;
    dto.comment_count = comment_count;
    Ok(Json(dto))
}
```

- [ ] **Step 4: Run tests + sqlx**

```bash
cd backend
cargo test --test photos_phase8b get_draft -- --nocapture
cargo test --test photos -- --nocapture
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add backend/src/photos/get.rs backend/tests/photos_phase8b.rs
git commit -m "feat(photos): detail endpoint 404s drafts for non-owners; expose is_draft/last_step/replaced_at"
```

---

## Task 6: Engagement endpoints — visibility on counts and comment list

**Files:**
- Modify: `backend/src/engagement/appreciations.rs` — `count` and `state_for_user` and the `appreciate`/`unappreciate` mutations.
- Modify: `backend/src/engagement/comments.rs` — `list` and `create`.

- [ ] **Step 1: Write the failing tests**

Append to `backend/tests/photos_phase8b.rs`:

```rust
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn appreciation_count_on_draft_404s_for_non_owner() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let bob = h.signup("b@e.com", "longenoughpw", "Bob").await;
    let photo_id = h.upload_draft(&alice).await;

    let status = h.get_status(
        &format!("/api/photos/{photo_id}/appreciations/count"),
        Some(&bob)
    ).await;
    assert_eq!(status, 404);

    let status_anon = h.get_status(
        &format!("/api/photos/{photo_id}/appreciations/count"),
        None
    ).await;
    assert_eq!(status_anon, 404);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn appreciate_a_draft_returns_404() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let bob = h.signup("b@e.com", "longenoughpw", "Bob").await;
    let photo_id = h.upload_draft(&alice).await;

    let status = h.post_status(
        &format!("/api/photos/{photo_id}/appreciate"),
        None,
        Some(&bob)
    ).await;
    assert_eq!(status, 404);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn comment_list_on_draft_404s_for_non_owner() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let bob = h.signup("b@e.com", "longenoughpw", "Bob").await;
    let photo_id = h.upload_draft(&alice).await;

    let status = h.get_status(
        &format!("/api/photos/{photo_id}/comments"),
        Some(&bob)
    ).await;
    assert_eq!(status, 404);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd backend && cargo test --test photos_phase8b appreciation_count_on_draft -- --nocapture`
Expected: FAIL.

- [ ] **Step 3: Add visibility checks to every public per-photo endpoint**

In `backend/src/engagement/appreciations.rs`, prepend a visibility check to each handler. The pattern:

```rust
use crate::auth::middleware::OptionalUser;
use crate::photos::queries::is_visible_to;

pub async fn count(
    State(state): State<AppState>,
    user: OptionalUser,
    Path(photo_id): Path<Uuid>,
) -> Result<Json<CountResponse>, AppError> {
    if !is_visible_to(&state.pool, photo_id, user.0.as_ref().map(|u| u.id)).await? {
        return Err(AppError::not_found("photo"));
    }
    // … existing body
}
```

Apply the same wrapper to `appreciate`, `unappreciate`, `state_for_user`. For mutations (`appreciate` / `unappreciate`), the viewer is `CurrentUser`, so pass `Some(user.0.id)`.

In `backend/src/engagement/comments.rs`, apply the same wrapper to `list` and `create`. (Reading `delete` operates on a comment id directly — the existing owner check stays.)

For each handler, the visibility check goes IMMEDIATELY before any other logic — no DB writes happen if the photo is invisible.

- [ ] **Step 4: Run tests**

```bash
cd backend
cargo test --test photos_phase8b -- --nocapture
cargo test --test engagement -- --nocapture
```
Expected: PASS for new tests; engagement tests still green (they create published photos via the existing test path which goes through `pipeline::process` → `mark_ready`; need to also set `published_at` for those tests to work — see next sub-step).

- [ ] **Step 5: Update Phase 5 `tests/engagement.rs` test fixtures to publish their photos**

The engagement tests upload via `POST /api/photos` and then exercise appreciate/comment. After Task 3, those uploads create drafts — the existing tests will start failing with 404 against the new visibility check. Update each test to call `POST /api/photos/:id/publish` after upload (publish endpoint doesn't exist yet — added in Task 7). For now, take the shortcut of `UPDATE photos SET published_at = now() WHERE id = $1` directly via the test pool inside the test setup. Add a helper near the top of `tests/engagement.rs`:

```rust
async fn publish_photo(pool: &sqlx::PgPool, id: uuid::Uuid) {
    sqlx::query!("update photos set published_at = now(), last_step = 'caption' where id = $1", id)
        .execute(pool).await.unwrap();
}
```

And call it after every photo upload in the engagement tests. Same in `tests/photos.rs` if any sub-test exercises a public endpoint other than `GET /api/photos/:id` for the owner.

- [ ] **Step 6: Commit**

```bash
git add backend/src/engagement/ backend/tests/ backend/.sqlx/
git commit -m "feat(engagement): drafts 404 on appreciate/comment/count via is_visible_to"
```

---

(Plan continues in subsequent tasks.)

## Task 7: Publish endpoint

**Files:**
- Create: `backend/src/photos/publish.rs`
- Modify: `backend/src/photos/mod.rs` (add `pub mod publish;`)
- Modify: `backend/src/http/mod.rs` (route)
- Test: append to `backend/tests/photos_phase8b.rs`.

`POST /api/photos/:id/publish` — owner-only, idempotent, sets `published_at = now()` and `last_step = 'caption'` on a draft. 400 if `status != 'ready'` (the binary must be decoded before publish). 200 on already-published (no change).

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn publish_sets_published_at_and_last_step_caption() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let id = h.upload_draft(&alice).await;
    h.wait_for_ready(id).await;

    let status = h.post_status(&format!("/api/photos/{id}/publish"), None, Some(&alice)).await;
    assert_eq!(status, 200);
    let row = sqlx::query!("select published_at, last_step from photos where id=$1", id)
        .fetch_one(&h.pool).await.unwrap();
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
    h.post_status(&format!("/api/photos/{id}/publish"), None, Some(&alice)).await;
    let first = sqlx::query_scalar!("select published_at from photos where id=$1", id)
        .fetch_one(&h.pool).await.unwrap().unwrap();

    let status = h.post_status(&format!("/api/photos/{id}/publish"), None, Some(&alice)).await;
    assert_eq!(status, 200);
    let second = sqlx::query_scalar!("select published_at from photos where id=$1", id)
        .fetch_one(&h.pool).await.unwrap().unwrap();
    assert_eq!(first, second, "publish must be idempotent — published_at unchanged");
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn publish_403_for_non_owner() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let bob = h.signup("b@e.com", "longenoughpw", "Bob").await;
    let id = h.upload_draft(&alice).await;
    h.wait_for_ready(id).await;
    let status = h.post_status(&format!("/api/photos/{id}/publish"), None, Some(&bob)).await;
    assert_eq!(status, 403);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn publish_400_when_status_processing() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let id = h.upload_draft(&alice).await;
    // Don't wait — pipeline still processing in the background.
    sqlx::query!("update photos set status='processing' where id=$1", id)
        .execute(&h.pool).await.unwrap();
    let status = h.post_status(&format!("/api/photos/{id}/publish"), None, Some(&alice)).await;
    assert_eq!(status, 400);
}
```

`H::wait_for_ready(id)` polls `select status from photos where id=$1` every 50 ms up to 10 s, returns when it sees `'ready'`. Add to the harness.

- [ ] **Step 2: Run to verify they fail**

Run: `cd backend && cargo test --test photos_phase8b publish_ -- --nocapture`
Expected: FAIL — route doesn't exist.

- [ ] **Step 3: Create `backend/src/photos/publish.rs`**

```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let row = sqlx::query!(
        "select owner_id, status, published_at from photos where id = $1",
        id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::not_found("photo"))?;

    if row.owner_id != user.id {
        return Err(AppError::Forbidden);
    }
    if row.published_at.is_some() {
        return Ok(StatusCode::OK); // idempotent no-op
    }
    if row.status != "ready" {
        return Err(AppError::Validation(
            "photo not ready: pipeline still processing or failed".into(),
        ));
    }
    sqlx::query!(
        "update photos set published_at = now(), last_step = 'caption' where id = $1",
        id
    )
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::OK)
}
```

- [ ] **Step 4: Wire module and route**

In `backend/src/photos/mod.rs` append `pub mod publish;`.

In `backend/src/http/mod.rs` add the route alongside the existing `/api/photos/:id` line:

```rust
.route(
    "/api/photos/:id/publish",
    post(crate::photos::publish::handler),
)
```

- [ ] **Step 5: Run tests**

```bash
cd backend
cargo sqlx prepare
cargo test --test photos_phase8b publish_ -- --nocapture
```
Expected: PASS.

- [ ] **Step 6: Replace the engagement test shortcut**

In `backend/tests/engagement.rs` (and `tests/photos.rs` if applicable), replace the inline `update photos set published_at` shortcut with calls to `POST /api/photos/:id/publish` so the test exercises the real codepath. Pattern:

```rust
async fn publish_photo(app: &Router, cookie: &str, id: Uuid) {
    let r = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri(format!("/api/photos/{id}/publish"))
            .header(header::COOKIE, cookie)
            .body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 200);
}
```

- [ ] **Step 7: Commit**

```bash
git add backend/src/photos/ backend/src/http/mod.rs backend/tests/ backend/.sqlx/
git commit -m "feat(photos): POST /api/photos/:id/publish (idempotent, owner-only)"
```

---

## Task 8: Metadata partial-update (PUT /api/photos/:id)

**Files:**
- Create: `backend/src/photos/metadata.rs`
- Modify: `backend/src/photos/mod.rs`, `backend/src/http/mod.rs`
- Test: append to `backend/tests/photos_phase8b.rs`.

Partial update of `target/caption/exif_json/last_step` plus the dedicated EXIF columns `taken_at/camera/lens/iso/exposure_s/focal_mm/ra_deg/dec_deg`. Works for both drafts and published. Owner-only. Free-form astro fields (`telescope/mount/filters/aperture/sessions/sensor_temp/gain`) round-trip through `exif_json`.

- [ ] **Step 1: Write the failing tests**

```rust
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
    let status = h.put_status(&format!("/api/photos/{draft}"), &body, Some(&alice)).await;
    assert_eq!(status, 200);
    let row = sqlx::query!(
        "select target, caption, iso, last_step from photos where id=$1", draft
    ).fetch_one(&h.pool).await.unwrap();
    assert_eq!(row.target.as_deref(), Some("M31"));
    assert_eq!(row.caption.as_deref(), Some("first light"));
    assert_eq!(row.iso, Some(1600));
    assert_eq!(row.last_step.as_deref(), Some("verify"));

    h.post_status(&format!("/api/photos/{draft}/publish"), None, Some(&alice)).await;
    let body2 = serde_json::json!({ "caption": "edited after publish" });
    let status = h.put_status(&format!("/api/photos/{draft}"), &body2, Some(&alice)).await;
    assert_eq!(status, 200);
    let row = sqlx::query!("select caption from photos where id=$1", draft)
        .fetch_one(&h.pool).await.unwrap();
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
    let status = h.put_status(&format!("/api/photos/{id}"), &body, Some(&bob)).await;
    assert_eq!(status, 403);
}
```

- [ ] **Step 2: Run to fail**

Run: `cd backend && cargo test --test photos_phase8b put_metadata -- --nocapture`
Expected: FAIL — no route.

- [ ] **Step 3: Create `backend/src/photos/metadata.rs`**

```rust
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

#[derive(Deserialize, Default)]
pub struct MetadataUpdate {
    pub target: Option<Option<String>>,
    pub caption: Option<Option<String>>,
    pub taken_at: Option<Option<DateTime<Utc>>>,
    pub camera: Option<Option<String>>,
    pub lens: Option<Option<String>>,
    pub iso: Option<Option<i32>>,
    pub exposure_s: Option<Option<f64>>,
    pub focal_mm: Option<Option<f64>>,
    pub exif_json: Option<serde_json::Value>,
    pub last_step: Option<String>,
}

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
    Json(patch): Json<MetadataUpdate>,
) -> Result<StatusCode, AppError> {
    let owner = sqlx::query_scalar!(
        "select owner_id from photos where id = $1", id
    )
    .fetch_optional(&state.pool).await?
    .ok_or(AppError::not_found("photo"))?;
    if owner != user.id {
        return Err(AppError::Forbidden);
    }

    if let Some(s) = &patch.last_step {
        if !["upload", "verify", "caption"].contains(&s.as_str()) {
            return Err(AppError::Validation(format!("bad last_step: {s}")));
        }
    }

    sqlx::query!(
        r#"
        update photos set
          target       = case when $2::bool then $3 else target end,
          caption      = case when $4::bool then $5 else caption end,
          taken_at     = case when $6::bool then $7 else taken_at end,
          camera       = case when $8::bool then $9 else camera end,
          lens         = case when $10::bool then $11 else lens end,
          iso          = case when $12::bool then $13 else iso end,
          exposure_s   = case when $14::bool then $15 else exposure_s end,
          focal_mm     = case when $16::bool then $17 else focal_mm end,
          exif_json    = case when $18::bool then $19 else exif_json end,
          last_step    = coalesce($20, last_step)
        where id = $1
        "#,
        id,
        patch.target.is_some(),    patch.target.flatten(),
        patch.caption.is_some(),   patch.caption.flatten(),
        patch.taken_at.is_some(),  patch.taken_at.flatten(),
        patch.camera.is_some(),    patch.camera.flatten(),
        patch.lens.is_some(),      patch.lens.flatten(),
        patch.iso.is_some(),       patch.iso.flatten(),
        patch.exposure_s.is_some(), patch.exposure_s.flatten(),
        patch.focal_mm.is_some(),  patch.focal_mm.flatten(),
        patch.exif_json.is_some(), patch.exif_json,
        patch.last_step.as_deref(),
    )
    .execute(&state.pool).await?;
    Ok(StatusCode::OK)
}
```

- [ ] **Step 4: Wire module + route**

`backend/src/photos/mod.rs`: `pub mod metadata;`

`backend/src/http/mod.rs`: extend the existing `/api/photos/:id` route to also accept PUT:

```rust
.route(
    "/api/photos/:id",
    get(crate::photos::get::handler).put(crate::photos::metadata::handler),
)
```

- [ ] **Step 5: Run tests**

```bash
cd backend
cargo sqlx prepare
cargo test --test photos_phase8b put_metadata -- --nocapture
```
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add backend/src/photos/ backend/src/http/mod.rs backend/tests/photos_phase8b.rs backend/.sqlx/
git commit -m "feat(photos): PUT /api/photos/:id partial metadata update (drafts and published)"
```

---

## Task 9: Pipeline `pipeline_options` + pending-deletes drain + error capture

**Files:**
- Modify: `backend/src/photos/pipeline.rs` — add `PipelineOptions`, drain pending deletes on success, capture pipeline error string on failure.
- Modify: `backend/src/photos/queries.rs` — `mark_failed` takes an error string; add `pending_deletes_for(photo_id)`, `delete_pending_deletes(photo_id)`.
- Modify: `backend/src/photos/upload.rs` — call `pipeline::finalize` with `PipelineOptions::Initial` (default).

Critical for Task 10 (replace) — replace will pass `PipelineOptions::Replace` to skip user-edited EXIF and to cause this drain branch to fire.

- [ ] **Step 1: Write the failing test**

```rust
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
```

- [ ] **Step 2: Run to fail**

Run: `cd backend && cargo test --test photos_phase8b mark_failed_records -- --nocapture`
Expected: FAIL — `mark_failed` arity mismatch.

- [ ] **Step 3: Update `mark_failed`**

In `backend/src/photos/queries.rs`:

```rust
pub async fn mark_failed(pool: &PgPool, id: Uuid, reason: &str) -> Result<(), AppError> {
    sqlx::query!(
        "update photos set status='failed', pipeline_error=$2 where id=$1",
        id, reason
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn pending_deletes_for(pool: &PgPool, photo_id: Uuid)
    -> Result<Vec<String>, AppError>
{
    let rows = sqlx::query_scalar!(
        "select storage_key from photo_pending_deletes where photo_id = $1",
        photo_id
    ).fetch_all(pool).await?;
    Ok(rows)
}

pub async fn drain_pending_deletes(pool: &PgPool, photo_id: Uuid) -> Result<(), AppError> {
    sqlx::query!("delete from photo_pending_deletes where photo_id = $1", photo_id)
        .execute(pool).await?;
    Ok(())
}
```

Update every caller of `mark_failed` (currently in `upload.rs` and `pipeline.rs::process`) to pass the error stringified: `&format!("{e}")`.

- [ ] **Step 4: Add `PipelineOptions` and drain branch**

In `backend/src/photos/pipeline.rs`:

```rust
#[derive(Clone, Copy, Debug)]
pub enum PipelineOptions {
    /// Initial upload — write all derived metadata.
    Initial,
    /// Replace — skip writing user-controlled fields (target/caption/exif),
    /// only refresh width/height/bytes; drain pending S3 deletes on success.
    Replace,
}

pub async fn finalize(
    pool: &PgPool,
    storage: Arc<dyn Storage>,
    photo_id: Uuid,
    bytes: Bytes,
    options: PipelineOptions,
) -> Result<(), AppError> {
    let bytes_for_blocking = bytes.clone();
    let parsed = tokio::task::spawn_blocking(move || {
        let exif_data = exif::parse_blocking(&bytes_for_blocking)?;
        let mut generated = Vec::with_capacity(THUMB_SIZES.len());
        for size in THUMB_SIZES {
            generated.push(thumbs::generate_blocking(&bytes_for_blocking, *size)?);
        }
        Ok::<_, AppError>((exif_data, generated))
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking join: {e}")))??;

    let (exif_data, generated) = parsed;
    let (full_w, full_h) = generated
        .iter()
        .max_by_key(|t| t.size)
        .map(|t| (t.width as i32, t.height as i32))
        .unwrap_or((0, 0));

    for thumb in generated {
        let key = format!("thumbs/{photo_id}/{}", thumb.size);
        let len = thumb.bytes.len() as i64;
        storage.put(&key, "image/jpeg", thumb.bytes).await?;
        queries::insert_thumbnail(pool, photo_id, thumb.size as i32, &key, len).await?;
    }

    match options {
        PipelineOptions::Initial => {
            queries::mark_ready(pool, photo_id, full_w, full_h, &exif_data).await?;
        }
        PipelineOptions::Replace => {
            // Skip user-edited EXIF/target/caption — only refresh size + bytes.
            queries::mark_ready_size_only(pool, photo_id, full_w, full_h).await?;
            // Drain any S3 keys queued for deferred deletion.
            let keys = queries::pending_deletes_for(pool, photo_id).await?;
            if !keys.is_empty() {
                storage.delete_objects(&keys).await?;
                queries::drain_pending_deletes(pool, photo_id).await?;
            }
        }
    }
    Ok(())
}
```

Add `mark_ready_size_only` to `queries.rs`:

```rust
pub async fn mark_ready_size_only(
    pool: &PgPool, id: Uuid, width: i32, height: i32,
) -> Result<(), AppError> {
    sqlx::query!(
        "update photos set status='ready', width=$2, height=$3, pipeline_error=null
         where id=$1",
        id, width, height
    ).execute(pool).await?;
    Ok(())
}
```

Update `mark_ready` (the Initial path) to also clear `pipeline_error = null` so a successful retry wipes the previous error.

Update `pipeline::process` to take `PipelineOptions` and pass it through.

Update the upload spawn in `backend/src/photos/upload.rs` to pass `PipelineOptions::Initial`:

```rust
if let Err(e) = pipeline::finalize(&pool, storage, id, bytes_clone, pipeline::PipelineOptions::Initial).await {
    let reason = format!("{e}");
    tracing::error!(photo_id=%id, error=%reason, "photo finalize failed");
    let _ = queries::mark_failed(&pool, id, &reason).await;
}
```

Same in the seed binary if it uses `pipeline::process`.

- [ ] **Step 5: Run all photo tests**

```bash
cd backend
cargo sqlx prepare
cargo test --test photos -- --nocapture
cargo test --test photos_phase8b -- --nocapture
```
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add backend/src/photos/ backend/.sqlx/ backend/tests/photos_phase8b.rs
git commit -m "feat(photos/pipeline): PipelineOptions enum, pipeline_error capture, drain pending deletes on Replace"
```

---

## Task 10: Replace endpoint

**Files:**
- Create: `backend/src/photos/replace.rs`
- Modify: `backend/src/photos/mod.rs`, `backend/src/http/mod.rs`
- Modify: `backend/src/photos/queries.rs` — add `enqueue_pending_deletes`, `delete_thumbnails_rows_for`, `swap_storage_key_for_replace`.
- Test: append to `backend/tests/photos_phase8b.rs`.

`POST /api/photos/:id/replace` — owner-only multipart. Stashes old master+thumb keys in `photo_pending_deletes`, swaps the master key + bytes + mime, sets `replaced_at = now()`, deletes old thumbnail rows from DB (S3 keys are stashed), spawns the pipeline with `PipelineOptions::Replace` which regenerates thumbs and drains pending deletes on success.

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn replace_swaps_storage_key_keeps_metadata() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let id = h.upload_draft(&alice).await;
    h.wait_for_ready(id).await;
    h.put_metadata(id, &alice, serde_json::json!({"target":"M31","caption":"v1"})).await;
    h.publish(id, &alice).await;

    let key_before: String = sqlx::query_scalar!("select storage_key from photos where id=$1", id)
        .fetch_one(&h.pool).await.unwrap();
    h.replace_with_jpeg(id, &alice).await;
    h.wait_for_ready(id).await;

    let row = sqlx::query!(
        "select storage_key, target, caption, replaced_at, published_at from photos where id=$1", id
    ).fetch_one(&h.pool).await.unwrap();
    assert_ne!(row.storage_key, key_before, "master key swapped");
    assert_eq!(row.target.as_deref(), Some("M31"), "target preserved");
    assert_eq!(row.caption.as_deref(), Some("v1"), "caption preserved");
    assert!(row.replaced_at.is_some());
    assert!(row.published_at.is_some(), "published_at preserved");
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
    sqlx::query!("update photos set status='processing' where id=$1", id)
        .execute(&h.pool).await.unwrap();
    let status = h.replace_status(id, &alice).await;
    assert_eq!(status, 400);
}
```

`H::replace_with_jpeg` posts a multipart with a fresh test JPEG to `/api/photos/:id/replace`; `H::replace_status` returns the status code only.

- [ ] **Step 2: Run to fail**

Run: `cd backend && cargo test --test photos_phase8b replace_ -- --nocapture`
Expected: FAIL — no route.

- [ ] **Step 3: Create `backend/src/photos/replace.rs`**

```rust
use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use bytes::Bytes;
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;
use crate::photos::{pipeline, queries};

const MAX_BYTES: usize = 50 * 1024 * 1024;
const ALLOWED_MIMES: &[&str] = &["image/jpeg", "image/png", "image/tiff"];

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let row = sqlx::query!(
        "select owner_id, status, storage_key from photos where id = $1", id
    )
    .fetch_optional(&state.pool).await?
    .ok_or(AppError::not_found("photo"))?;
    if row.owner_id != user.id {
        return Err(AppError::Forbidden);
    }
    if row.status == "processing" {
        return Err(AppError::Validation("pipeline busy".into()));
    }

    let mut file_bytes: Option<Bytes> = None;
    let mut filename = String::from("upload.bin");
    let mut mime = String::from("application/octet-stream");
    while let Some(field) = multipart.next_field().await
        .map_err(|e| AppError::Validation(format!("multipart: {e}")))?
    {
        if field.name() == Some("file") {
            if let Some(n) = field.file_name() { filename = n.to_string(); }
            if let Some(c) = field.content_type() { mime = c.to_string(); }
            let data = field.bytes().await
                .map_err(|e| AppError::Validation(format!("read: {e}")))?;
            if data.len() > MAX_BYTES {
                return Err(AppError::Validation(format!(
                    "file too large: {} bytes (max {MAX_BYTES})", data.len()
                )));
            }
            file_bytes = Some(data);
        }
    }
    let bytes = file_bytes.ok_or_else(|| AppError::Validation("missing file".into()))?;
    if !ALLOWED_MIMES.contains(&mime.as_str()) {
        return Err(AppError::Validation(format!("unsupported mime: {mime}")));
    }

    // 1. Stash old master + thumb keys for deferred deletion.
    let mut to_stash = vec![row.storage_key.clone()];
    let old_thumb_keys: Vec<String> = sqlx::query_scalar!(
        "select storage_key from thumbnails where photo_id = $1", id
    ).fetch_all(&state.pool).await?;
    to_stash.extend(old_thumb_keys);
    queries::enqueue_pending_deletes(&state.pool, id, &to_stash).await?;

    // 2. Upload new master to a fresh key.
    let new_key = format!("originals/{}", Uuid::new_v4());
    state.storage.put(&new_key, &mime, bytes.clone()).await?;

    // 3. Atomically swap key + size + mime + replaced_at + status='processing'.
    queries::swap_storage_key_for_replace(
        &state.pool, id, &new_key, &filename, &mime, bytes.len() as i64,
    ).await?;

    // 4. DELETE old thumbnail rows (S3 keys already stashed).
    sqlx::query!("delete from thumbnails where photo_id = $1", id)
        .execute(&state.pool).await?;

    // 5. Spawn pipeline with Replace options — drains pending deletes on success.
    let pool = state.pool.clone();
    let storage = state.storage.clone();
    tokio::spawn(async move {
        if let Err(e) =
            pipeline::finalize(&pool, storage, id, bytes, pipeline::PipelineOptions::Replace).await
        {
            let reason = format!("{e}");
            tracing::error!(photo_id=%id, error=%reason, "replace finalize failed");
            let _ = queries::mark_failed(&pool, id, &reason).await;
        }
    });

    Ok(StatusCode::ACCEPTED)
}
```

- [ ] **Step 4: Add the queries**

In `backend/src/photos/queries.rs`:

```rust
pub async fn enqueue_pending_deletes(
    pool: &PgPool, photo_id: Uuid, storage_keys: &[String],
) -> Result<(), AppError> {
    if storage_keys.is_empty() { return Ok(()); }
    for key in storage_keys {
        sqlx::query!(
            "insert into photo_pending_deletes (photo_id, storage_key) values ($1, $2)",
            photo_id, key
        ).execute(pool).await?;
    }
    Ok(())
}

pub async fn swap_storage_key_for_replace(
    pool: &PgPool, id: Uuid, new_key: &str, original_name: &str,
    mime: &str, bytes: i64,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        update photos set
          storage_key = $2,
          original_name = $3,
          mime = $4,
          bytes = $5,
          status = 'processing',
          replaced_at = now(),
          pipeline_error = null
        where id = $1
        "#,
        id, new_key, original_name, mime, bytes
    ).execute(pool).await?;
    Ok(())
}
```

- [ ] **Step 5: Wire module + route**

`backend/src/photos/mod.rs`: `pub mod replace;`

`backend/src/http/mod.rs`:

```rust
.route(
    "/api/photos/:id/replace",
    post(crate::photos::replace::handler)
        .layer(axum::extract::DefaultBodyLimit::max(50 * 1024 * 1024)),
)
```

- [ ] **Step 6: Run all tests**

```bash
cd backend
cargo sqlx prepare
cargo test --test photos_phase8b -- --nocapture
cargo test --test photos -- --nocapture
```
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add backend/src/photos/ backend/src/http/mod.rs backend/.sqlx/ backend/tests/photos_phase8b.rs
git commit -m "feat(photos): POST /api/photos/:id/replace with deferred S3 deletion"
```

---

## Task 11: `me::stats` endpoint

**Files:**
- Create: `backend/src/users/stats.rs`
- Modify: `backend/src/users/mod.rs`, `backend/src/http/mod.rs`
- Test: append to `backend/tests/photos_phase8b.rs`.

`GET /api/me/stats` returns `{published_count, draft_count, integration_secs, appreciations_received}` for the current user. Drafts excluded from `integration_secs` and `appreciations_received` (drafts can't accumulate appreciations anyway, but we ensure it explicitly).

- [ ] **Step 1: Write the failing test**

```rust
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn me_stats_counts_published_and_drafts_separately() {
    let h = harness().await;
    let alice = h.signup("a@e.com", "longenoughpw", "Alice").await;
    let p1 = h.upload_draft(&alice).await; h.wait_for_ready(p1).await; h.publish(p1, &alice).await;
    let p2 = h.upload_draft(&alice).await; h.wait_for_ready(p2).await; h.publish(p2, &alice).await;
    let _draft = h.upload_draft(&alice).await;

    let body = h.get_json("/api/me/stats", Some(&alice)).await;
    assert_eq!(body["published_count"], 2);
    assert_eq!(body["draft_count"], 1);
    assert_eq!(body["appreciations_received"], 0);
}
```

- [ ] **Step 2: Run to fail**

Run: `cd backend && cargo test --test photos_phase8b me_stats -- --nocapture`
Expected: FAIL — no route.

- [ ] **Step 3: Create `backend/src/users/stats.rs`**

```rust
use axum::{Json, extract::State};
use serde::Serialize;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

#[derive(Serialize)]
pub struct MeStats {
    pub published_count: i64,
    pub draft_count: i64,
    pub integration_secs: f64,
    pub appreciations_received: i64,
}

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<MeStats>, AppError> {
    let row = sqlx::query!(
        r#"
        select
          count(*) filter (where published_at is not null) as "pub!",
          count(*) filter (where published_at is null)     as "drafts!",
          coalesce(sum(exposure_s) filter (where published_at is not null), 0)::float8
            as "integ!"
        from photos
        where owner_id = $1
        "#,
        user.id
    ).fetch_one(&state.pool).await?;

    let appreciations_received = sqlx::query_scalar!(
        r#"
        select count(*) as "c!"
        from appreciations a
        join photos p on p.id = a.photo_id
        where p.owner_id = $1 and p.published_at is not null
        "#,
        user.id
    ).fetch_one(&state.pool).await?;

    Ok(Json(MeStats {
        published_count: row.pub_,
        draft_count: row.drafts,
        integration_secs: row.integ,
        appreciations_received,
    }))
}
```

(If the field-name colon-syntax with reserved keyword fails, alias as `pub_count` in SQL and rename here.)

- [ ] **Step 4: Wire**

`backend/src/users/mod.rs`: `pub mod stats;`

`backend/src/http/mod.rs`:

```rust
.route("/api/me/stats", get(crate::users::stats::handler))
```

- [ ] **Step 5: Run + commit**

```bash
cd backend
cargo sqlx prepare
cargo test --test photos_phase8b me_stats -- --nocapture
git add backend/src/users/ backend/src/http/mod.rs backend/.sqlx/ backend/tests/photos_phase8b.rs
git commit -m "feat(users): GET /api/me/stats — published/drafts/integration/appreciations"
```

---

## Task 12: Hourly purge worker — drain stale `photo_pending_deletes`

**Files:**
- Modify: `backend/src/jobs/purge_deletions.rs` — add a second pass that sweeps `photo_pending_deletes` rows older than 7 days (orphaned because the pipeline never succeeded).
- Test: extend the existing test for the worker if any; otherwise add `tests/photos_phase8b.rs` test.

- [ ] **Step 1: Write the failing test**

```rust
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn purge_worker_sweeps_pending_deletes_older_than_7_days() {
    let (pool, _pg) = test_pool().await;
    let storage = std::sync::Arc::new(astrophoto::storage::MemoryStorage::new());
    let owner = Uuid::new_v4();
    sqlx::query!(
        "insert into users (id, email, password_hash, display_name)
         values ($1, $2, '', 'O')", owner, format!("o-{owner}@e")
    ).execute(&pool).await.unwrap();
    let id = sqlx::query_scalar!(
        "insert into photos (owner_id, storage_key, original_name, bytes, mime,
                             status, original_uploaded_at, last_step)
         values ($1, 'k', 'n.jpg', 10, 'image/jpeg', 'failed', now(), 'upload')
         returning id", owner
    ).fetch_one(&pool).await.unwrap();
    storage.put("orphan-key", "image/jpeg", bytes::Bytes::from_static(b"x")).await.unwrap();
    sqlx::query!(
        "insert into photo_pending_deletes (photo_id, storage_key, queued_at)
         values ($1, 'orphan-key', now() - interval '8 days')", id
    ).execute(&pool).await.unwrap();

    astrophoto::jobs::purge_deletions::sweep_pending_deletes(&pool, storage.as_ref())
        .await.unwrap();

    let remaining: i64 = sqlx::query_scalar!(
        r#"select count(*) as "c!" from photo_pending_deletes where storage_key='orphan-key'"#
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(remaining, 0);
    assert!(storage.get("orphan-key").await.unwrap().is_none(), "S3 object swept");
}
```

- [ ] **Step 2: Run to fail**

Run: `cd backend && cargo test --test photos_phase8b purge_worker_sweeps -- --nocapture`
Expected: FAIL — function doesn't exist.

- [ ] **Step 3: Add `sweep_pending_deletes` and call from `purge_once`**

In `backend/src/jobs/purge_deletions.rs`:

```rust
pub async fn sweep_pending_deletes(
    pool: &PgPool, storage: &dyn Storage,
) -> Result<u64, AppError> {
    let stale: Vec<String> = sqlx::query_scalar!(
        "select storage_key from photo_pending_deletes
         where queued_at < now() - interval '7 days'"
    ).fetch_all(pool).await?;
    if stale.is_empty() {
        return Ok(0);
    }
    storage.delete_objects(&stale).await?;
    let n = sqlx::query!(
        "delete from photo_pending_deletes where queued_at < now() - interval '7 days'"
    ).execute(pool).await?.rows_affected();
    Ok(n)
}
```

Call it from `purge_once` after the user-purge loop:

```rust
pub async fn purge_once(pool: &PgPool, storage: &dyn Storage) -> Result<u64, AppError> {
    // … existing user-purge loop …

    // Sweep orphaned pending S3 deletes (replace pipeline never reached 'ready').
    if let Err(e) = sweep_pending_deletes(pool, storage).await {
        tracing::error!(error = ?e, "sweep_pending_deletes failed");
    }

    tracing::info!(deleted, total_due = due.len(), "purge cycle done");
    Ok(deleted)
}
```

- [ ] **Step 4: Run + commit**

```bash
cd backend
cargo sqlx prepare
cargo test --test photos_phase8b purge_worker -- --nocapture
git add backend/src/jobs/purge_deletions.rs backend/.sqlx/ backend/tests/photos_phase8b.rs
git commit -m "feat(jobs): hourly worker sweeps photo_pending_deletes >7d old"
```

---

## Task 13: Re-export TS types

**Files:**
- Modify: `backend/src/api_types.rs` — add `MeStats` (TS type) and a `PhotoDetail` mirror so the frontend can import the new fields with type safety. (The handler-side `PhotoDetail` in `backend/src/photos/get.rs` is not the ts-rs source of truth; mirror only the fields the frontend reads.)

- [ ] **Step 1: Append to `backend/src/api_types.rs`**

```rust
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "MeStats.ts")]
pub struct MeStats {
    pub published_count: i64,
    pub draft_count: i64,
    pub integration_secs: f64,
    pub appreciations_received: i64,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "PhotoDetail.ts")]
pub struct PhotoDetail {
    pub id: String,
    pub owner_id: String,
    pub status: String,
    pub original_name: String,
    pub bytes: i64,
    pub mime: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub iso: Option<i32>,
    pub exposure_s: Option<f64>,
    pub focal_mm: Option<f64>,
    pub target: Option<String>,
    pub caption: Option<String>,
    pub taken_at: Option<String>,
    pub created_at: String,
    pub appreciation_count: i64,
    pub comment_count: i64,
    pub is_draft: bool,
    pub last_step: Option<String>,
    pub replaced_at: Option<String>,
    pub original_uploaded_at: String,
    pub pipeline_error: Option<String>,
}
```

- [ ] **Step 2: Regenerate frontend types**

```bash
just types
```

- [ ] **Step 3: Commit**

```bash
git add backend/src/api_types.rs frontend/src/lib/api/types.ts frontend/src/lib/api/bindings/
git commit -m "feat(types): MeStats + PhotoDetail TS exports for Phase 8b"
```

---

## Task 14: Frontend — `PhotoTitle` component (untitled fallback)

**Files:**
- Create: `frontend/src/lib/components/photos/PhotoTitle.svelte`

The smallest, most-reused new component. Every other surface depends on it.

- [ ] **Step 1: Write the component**

```svelte
<script lang="ts">
  let {
    photo,
    size = 'md'
  }: {
    photo: { target?: string | null; original_name: string };
    size?: 'sm' | 'md' | 'lg';
  } = $props();
</script>

{#if photo.target}
  <span class="title size-{size}">{photo.target}</span>
{:else}
  <span class="title untitled size-{size}">{photo.original_name}</span>
  <em class="untitled-chip">UNTITLED</em>
{/if}

<style>
  .title { color: var(--fg-primary); }
  .title.size-sm { font-size: 14px; }
  .title.size-md { font-size: 18px; }
  .title.size-lg { font-family: var(--font-display); font-size: 36px; }
  .untitled { font-style: italic; color: var(--fg-secondary); }
  .untitled-chip {
    display: inline-block;
    margin-left: 8px;
    padding: 2px 6px;
    border: 1px dashed var(--border-default);
    border-radius: 2px;
    font-family: var(--font-mono);
    font-size: 9px;
    letter-spacing: 0.1em;
    color: var(--fg-muted);
    text-transform: uppercase;
    font-style: normal;
    vertical-align: middle;
  }
</style>
```

- [ ] **Step 2: Commit (used by later tasks)**

```bash
git add frontend/src/lib/components/photos/PhotoTitle.svelte
git commit -m "feat(frontend): PhotoTitle component with UNTITLED chip fallback"
```

---

## Task 15: Frontend — split `/upload` into Step 01 only (redirect to verify)

**Files:**
- Modify: `frontend/src/routes/upload/+page.server.ts` — POST returns id; redirect to `/upload/{id}/verify`.
- Modify: `frontend/src/routes/upload/+page.svelte` — drop right-column form (target/caption/submit); keep only file picker. Submit button label "Continue →".

- [ ] **Step 1: Replace `+page.server.ts`**

```ts
import { fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ locals }) => {
  if (!locals.user) redirect(303, '/signin');
  return {};
};

export const actions: Actions = {
  default: async ({ request, fetch, cookies }) => {
    const data = await request.formData();
    const file = data.get('file');
    if (!(file instanceof File) || file.size === 0) {
      return fail(400, { message: 'Choose a file to upload.' });
    }
    if (file.size > 50 * 1024 * 1024) {
      return fail(413, { message: 'File too large (max 50 MB).' });
    }
    const forwarded = new FormData();
    forwarded.append('file', file, file.name);
    const cookie = cookies.getAll().map((c) => `${c.name}=${c.value}`).join('; ');
    let res: Response;
    try {
      res = await fetch(`${API}/api/photos`, {
        method: 'POST', headers: { Cookie: cookie }, body: forwarded
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Network error';
      return fail(503, { message: `Backend unreachable: ${msg}` });
    }
    if (!res.ok) {
      if (res.status === 401) return fail(401, { message: 'Sign in required.' });
      const txt = await res.text();
      return fail(500, { message: `Upload failed: ${txt}` });
    }
    const body = (await res.json()) as { id: string; status: string };
    redirect(303, `/upload/${body.id}/verify`);
  }
};
```

- [ ] **Step 2: Trim `+page.svelte`**

In `frontend/src/routes/upload/+page.svelte`:
- Remove the entire right-column block (the `<div class="col-right">…</div>` containing TARGET / CAPTION / submit).
- Replace the action button row (`.actions`) with a single primary submit button labelled `Continue →`.
- Remove the `caption`/`target` form fields entirely.

The file picker on the left stays as-is.

- [ ] **Step 3: Visual smoke test**

```bash
just dev
# Browse to http://localhost:5173/upload, drop a JPEG, verify the page
# transitions to /upload/{id}/verify (404 expected for now — Task 16 builds it).
```

- [ ] **Step 4: Commit**

```bash
git add frontend/src/routes/upload/
git commit -m "feat(upload): Step 01 simplified to file picker; redirects to /upload/[id]/verify"
```

---

## Task 16: Frontend — `/upload/[id]/verify` (Step 02)

**Files:**
- Create: `frontend/src/routes/upload/[id]/verify/+page.server.ts`
- Create: `frontend/src/routes/upload/[id]/verify/+page.svelte`
- Modify: `frontend/src/lib/api/client.ts` — add `getPhoto(id)`, `putPhotoMetadata(id, patch)`, `publishPhoto(id)`.

- [ ] **Step 1: Extend the API client**

In `frontend/src/lib/api/client.ts`:

```ts
async function getPhoto(id: string, fetchFn: typeof fetch = fetch) {
  const r = await fetchFn(`${API_BASE}/api/photos/${id}`, { credentials: 'include' });
  if (!r.ok) throw new ApiError(r.status, await r.text());
  return (await r.json()) as PhotoDetail;
}

async function putPhotoMetadata(id: string, patch: Record<string, unknown>, fetchFn = fetch) {
  const r = await fetchFn(`${API_BASE}/api/photos/${id}`, {
    method: 'PUT', credentials: 'include',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(patch)
  });
  if (!r.ok) throw new ApiError(r.status, await r.text());
}

async function publishPhoto(id: string, fetchFn = fetch) {
  const r = await fetchFn(`${API_BASE}/api/photos/${id}/publish`, {
    method: 'POST', credentials: 'include'
  });
  if (!r.ok) throw new ApiError(r.status, await r.text());
}
```

Add `getPhoto, putPhotoMetadata, publishPhoto` to the `api` export object. Import `PhotoDetail` from `./bindings/PhotoDetail`.

- [ ] **Step 2: Create `+page.server.ts`**

```ts
import { error, fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ params, locals, fetch, cookies }) => {
  if (!locals.user) redirect(303, '/signin');
  const cookie = cookies.getAll().map((c) => `${c.name}=${c.value}`).join('; ');
  const r = await fetch(`${API}/api/photos/${params.id}`, { headers: { Cookie: cookie } });
  if (r.status === 404) error(404, 'Photo not found');
  if (!r.ok) error(500, 'Backend error');
  const photo = await r.json();
  if (photo.owner_id !== locals.user.id) error(404, 'Not found');
  return { photo };
};

async function callPut(
  fetch: typeof globalThis.fetch, cookie: string, id: string, patch: Record<string, unknown>
) {
  const r = await fetch(`${API}/api/photos/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json', Cookie: cookie },
    body: JSON.stringify(patch)
  });
  return r;
}

export const actions: Actions = {
  save_continue: async ({ request, params, fetch, cookies }) => {
    const fd = await request.formData();
    const patch = collectPatch(fd, 'verify');
    const cookie = cookies.getAll().map((c) => `${c.name}=${c.value}`).join('; ');
    const r = await callPut(fetch, cookie, params.id!, patch);
    if (!r.ok) return fail(r.status, { error: await r.text() });
    redirect(303, `/upload/${params.id}/caption`);
  },
  save_draft: async ({ request, params, fetch, cookies }) => {
    const fd = await request.formData();
    const patch = collectPatch(fd, 'verify');
    const cookie = cookies.getAll().map((c) => `${c.name}=${c.value}`).join('; ');
    await callPut(fetch, cookie, params.id!, patch);
    redirect(303, '/account/frames');
  },
  save_changes_published: async ({ request, params, fetch, cookies }) => {
    // Edit-metadata terminus: published photo, save and go to /photo/[slug].
    const fd = await request.formData();
    const patch = collectPatch(fd, 'caption');
    const cookie = cookies.getAll().map((c) => `${c.name}=${c.value}`).join('; ');
    await callPut(fetch, cookie, params.id!, patch);
    redirect(303, `/photo/${params.id}`);
  }
};

function collectPatch(fd: FormData, last_step: 'verify' | 'caption') {
  const numOrNull = (k: string): number | null => {
    const v = fd.get(k);
    if (typeof v !== 'string' || v.trim() === '') return null;
    const n = Number(v);
    return Number.isFinite(n) ? n : null;
  };
  const strOrNull = (k: string): string | null => {
    const v = fd.get(k);
    return typeof v === 'string' && v.trim() !== '' ? v.trim() : null;
  };
  return {
    target: strOrNull('target'),
    taken_at: strOrNull('taken_at'),
    camera: strOrNull('camera'),
    lens: strOrNull('lens'),
    iso: numOrNull('iso'),
    exposure_s: numOrNull('exposure_s'),
    focal_mm: numOrNull('focal_mm'),
    last_step
  };
}
```

- [ ] **Step 3: Create `+page.svelte`**

```svelte
<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  let polling = $state<number | null>(null);

  let isPublished = $derived(!data.photo.is_draft);
  let isProcessing = $derived(data.photo.status === 'processing');
  let isFailed = $derived(data.photo.status === 'failed');

  $effect(() => {
    if (isProcessing && polling === null) {
      polling = window.setInterval(() => invalidateAll(), 2000);
    }
    if (!isProcessing && polling !== null) {
      clearInterval(polling); polling = null;
    }
    return () => { if (polling !== null) clearInterval(polling); };
  });
</script>

<svelte:head><title>Verify data — Astrophoto</title></svelte:head>
<AppHeader active="Gallery" />

<div class="verify-page">
  <div class="t-eyebrow">{isPublished ? 'EDIT METADATA' : 'NEW FRAME · STEP 02'}</div>
  <h1 class="title">Verify the <em>data</em>.</h1>

  {#if isFailed}
    <div class="panel-failed">
      <div class="t-eyebrow danger">● UPLOAD FAILED · {data.photo.pipeline_error ?? 'unknown error'}</div>
      <div class="actions">
        <form method="POST" action="?/save_draft"><Button variant="ghost" type="submit">Discard</Button></form>
        <Button variant="primary" href="/upload">Retry upload</Button>
      </div>
    </div>
  {:else}
    <form method="POST" action={isPublished ? '?/save_changes_published' : '?/save_continue'} class="metadata-form">
      <fieldset disabled={isProcessing}>
        <div class="grid">
          <label><span class="t-label">TARGET</span>
            <Input name="target" value={data.photo.target ?? ''} placeholder="M31, NGC 7000…" /></label>
          <label><span class="t-label">CAMERA</span>
            <Input name="camera" value={data.photo.camera ?? ''} /></label>
          <label><span class="t-label">LENS</span>
            <Input name="lens" value={data.photo.lens ?? ''} /></label>
          <label><span class="t-label">ISO</span>
            <Input type="number" name="iso" value={data.photo.iso?.toString() ?? ''} /></label>
          <label><span class="t-label">EXPOSURE (S)</span>
            <Input type="number" step="0.01" name="exposure_s"
              value={data.photo.exposure_s?.toString() ?? ''} /></label>
          <label><span class="t-label">FOCAL (MM)</span>
            <Input type="number" name="focal_mm" value={data.photo.focal_mm?.toString() ?? ''} /></label>
        </div>
      </fieldset>

      {#if isProcessing}
        <p class="t-meta">● PROCESSING THUMBNAILS — polling every 2 s</p>
      {/if}
      {#if form?.error}
        <p class="t-meta form-error">{form.error}</p>
      {/if}

      <div class="actions">
        {#if isPublished}
          <Button variant="ghost" href="/upload/{data.photo.id}/caption">Edit caption →</Button>
          <Button variant="primary" type="submit" disabled={isProcessing}>Save changes</Button>
        {:else}
          <Button variant="ghost" type="submit" formaction="?/save_draft" disabled={isProcessing}>Save as draft</Button>
          <Button variant="primary" type="submit" disabled={isProcessing}>Continue →</Button>
        {/if}
      </div>
    </form>
  {/if}
</div>

<style>
  .verify-page { padding: 40px 64px; max-width: 960px; margin: 0 auto; }
  .title { font-family: var(--font-display); font-size: 44px; margin: 8px 0 32px; }
  .title em { font-style: italic; }
  .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px 24px; }
  .grid label { display: flex; flex-direction: column; gap: 6px; }
  .actions { display: flex; gap: 12px; justify-content: flex-end; margin-top: 32px; }
  .panel-failed { padding: 24px; border: 1px solid var(--danger); margin-top: 32px; }
  .danger { color: var(--danger); }
  .form-error { color: var(--danger); }
  @media (max-width: 768px) { .verify-page { padding: 32px 24px; } .grid { grid-template-columns: 1fr; } }
</style>
```

- [ ] **Step 4: Manually verify the flow**

```bash
just dev
# /upload → drop JPEG → redirect to /upload/{id}/verify → poll resolves to ready → fill target → Continue → 404 on /caption (next task)
```

- [ ] **Step 5: Commit**

```bash
git add frontend/src/routes/upload/\[id\]/verify/ frontend/src/lib/api/client.ts
git commit -m "feat(upload): Step 02 verify page (processing/ready/failed branches, metadata form, Edit-metadata terminus)"
```

---

## Task 17: Frontend — `/upload/[id]/caption` (Step 03)

**Files:**
- Create: `frontend/src/routes/upload/[id]/caption/+page.server.ts`
- Create: `frontend/src/routes/upload/[id]/caption/+page.svelte`

- [ ] **Step 1: Create `+page.server.ts`**

```ts
import { error, fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ params, locals, fetch, cookies }) => {
  if (!locals.user) redirect(303, '/signin');
  const cookie = cookies.getAll().map((c) => `${c.name}=${c.value}`).join('; ');
  const r = await fetch(`${API}/api/photos/${params.id}`, { headers: { Cookie: cookie } });
  if (r.status === 404) error(404, 'Photo not found');
  if (!r.ok) error(500, 'Backend error');
  const photo = await r.json();
  if (photo.owner_id !== locals.user.id) error(404, 'Not found');
  return { photo };
};

export const actions: Actions = {
  publish: async ({ request, params, fetch, cookies }) => {
    const fd = await request.formData();
    const caption = String(fd.get('caption') ?? '').trim();
    const cookie = cookies.getAll().map((c) => `${c.name}=${c.value}`).join('; ');
    let r = await fetch(`${API}/api/photos/${params.id}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', Cookie: cookie },
      body: JSON.stringify({ caption: caption || null, last_step: 'caption' })
    });
    if (!r.ok) return fail(r.status, { error: await r.text() });
    r = await fetch(`${API}/api/photos/${params.id}/publish`, {
      method: 'POST', headers: { Cookie: cookie }
    });
    if (!r.ok) return fail(r.status, { error: await r.text() });
    redirect(303, `/photo/${params.id}`);
  },
  save_draft: async ({ request, params, fetch, cookies }) => {
    const fd = await request.formData();
    const caption = String(fd.get('caption') ?? '').trim();
    const cookie = cookies.getAll().map((c) => `${c.name}=${c.value}`).join('; ');
    await fetch(`${API}/api/photos/${params.id}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', Cookie: cookie },
      body: JSON.stringify({ caption: caption || null, last_step: 'caption' })
    });
    redirect(303, '/account/frames');
  },
  save_changes: async ({ request, params, fetch, cookies }) => {
    // Already-published variant: caption-only PUT, no publish call.
    const fd = await request.formData();
    const caption = String(fd.get('caption') ?? '').trim();
    const cookie = cookies.getAll().map((c) => `${c.name}=${c.value}`).join('; ');
    await fetch(`${API}/api/photos/${params.id}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json', Cookie: cookie },
      body: JSON.stringify({ caption: caption || null })
    });
    redirect(303, `/photo/${params.id}`);
  }
};
```

- [ ] **Step 2: Create `+page.svelte`**

```svelte
<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import Button from '$lib/components/Button.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  let isPublished = $derived(!data.photo.is_draft);
</script>

<svelte:head><title>Caption — Astrophoto</title></svelte:head>
<AppHeader active="Gallery" />

<div class="caption-page">
  <div class="t-eyebrow">{isPublished ? 'EDIT CAPTION' : 'NEW FRAME · STEP 03'}</div>
  <h1 class="title">{isPublished ? 'Edit the caption.' : 'Add a caption.'}</h1>

  <div class="recap">
    <div><span class="t-label">TARGET</span> {data.photo.target ?? '—'}</div>
    <div><span class="t-label">CAMERA</span> {data.photo.camera ?? '—'}</div>
    <div><span class="t-label">EXPOSURE</span> {data.photo.exposure_s ?? '—'} s</div>
  </div>

  <form method="POST" action={isPublished ? '?/save_changes' : '?/publish'} class="caption-form">
    <Textarea name="caption" rows={8} value={data.photo.caption ?? ''}
      placeholder="Describe the conditions, processing, equipment used…" />
    {#if form?.error}<p class="t-meta form-error">{form.error}</p>{/if}
    <div class="actions">
      {#if !isPublished}
        <Button variant="ghost" type="submit" formaction="?/save_draft">Save as draft</Button>
      {/if}
      <Button variant="primary" type="submit">{isPublished ? 'Save changes' : 'Publish'}</Button>
    </div>
  </form>
</div>

<style>
  .caption-page { padding: 40px 64px; max-width: 720px; margin: 0 auto; }
  .title { font-family: var(--font-display); font-size: 44px; margin: 8px 0 32px; }
  .recap { display: flex; gap: 32px; padding: 16px; background: var(--bg-surface); margin-bottom: 24px; }
  .recap > div { display: flex; flex-direction: column; gap: 4px; }
  .actions { display: flex; gap: 12px; justify-content: flex-end; margin-top: 24px; }
  .form-error { color: var(--danger); }
  @media (max-width: 768px) { .caption-page { padding: 32px 24px; } .recap { flex-direction: column; gap: 8px; } }
</style>
```

- [ ] **Step 3: Manual smoke + commit**

```bash
just dev
# Walk a draft from /upload → /upload/{id}/verify → Continue → /upload/{id}/caption → Publish → /photo/{id}.
git add frontend/src/routes/upload/\[id\]/caption/
git commit -m "feat(upload): Step 03 caption page (publish flow + already-published Save changes branch)"
```

---

## Task 18: Frontend — `/account/frames` and the new component primitives

**Files:**
- Create:
  - `frontend/src/routes/account/frames/+page.server.ts`
  - `frontend/src/routes/account/frames/+page.svelte`
  - `frontend/src/routes/account/frames/drafts/+page.server.ts` (thin redirect)
  - `frontend/src/lib/components/photos/StatsRow.svelte`
  - `frontend/src/lib/components/photos/FilterChips.svelte`
  - `frontend/src/lib/components/photos/DraftsCallout.svelte`
  - `frontend/src/lib/components/photos/DraftCard.svelte`
  - `frontend/src/lib/components/photos/PhotosTable.svelte`

(Group the components in this task because they are mutually inert chrome with no logic — co-locating reduces back-and-forth.)

- [ ] **Step 1: Create `StatsRow.svelte`**

```svelte
<script lang="ts">
  let { stats }: {
    stats: {
      published_count: number; draft_count: number;
      integration_secs: number; appreciations_received: number;
    }
  } = $props();

  function formatHours(secs: number): string {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    return `${h} h ${m} m`;
  }
</script>

<div class="stats-row">
  <div class="cell"><span class="t-label">PUBLISHED</span><span class="num">{stats.published_count}</span></div>
  <div class="cell" class:accent={stats.draft_count > 0}>
    <span class="t-label">DRAFTS</span><span class="num">{stats.draft_count}</span>
  </div>
  <div class="cell"><span class="t-label">TOTAL INTEGRATION</span><span class="num">{formatHours(stats.integration_secs)}</span></div>
  <div class="cell"><span class="t-label">APPRECIATIONS</span><span class="num">{stats.appreciations_received}</span></div>
</div>

<style>
  .stats-row { display: flex; gap: 32px; }
  .cell { display: flex; flex-direction: column; gap: 4px; align-items: flex-end; }
  .num { font-family: var(--font-display); font-size: 24px; }
  .cell.accent .num { color: var(--accent); }
</style>
```

- [ ] **Step 2: Create `FilterChips.svelte`**

```svelte
<script lang="ts">
  let { active, counts, sort, view }: {
    active: 'all' | 'published' | 'drafts';
    counts: { all: number; published: number; drafts: number };
    sort: 'newest' | 'oldest';
    view: 'list' | 'grid';
  } = $props();

  function href(filter: string) {
    const p = new URLSearchParams({ filter, sort, view });
    return `/account/frames?${p.toString()}`;
  }
  function sortHref(s: string) {
    const p = new URLSearchParams({ filter: active, sort: s, view });
    return `/account/frames?${p.toString()}`;
  }
  function viewHref(v: string) {
    const p = new URLSearchParams({ filter: active, sort, view: v });
    return `/account/frames?${p.toString()}`;
  }
</script>

<div class="filter-bar">
  <div class="chips">
    <a href={href('all')} class:on={active === 'all'}>All · {counts.all}</a>
    <a href={href('published')} class:on={active === 'published'}>Published · {counts.published}</a>
    <a href={href('drafts')} class:on={active === 'drafts'}>Drafts · {counts.drafts}</a>
  </div>
  <div class="controls">
    <a href={sortHref(sort === 'newest' ? 'oldest' : 'newest')} class="t-meta">
      SORT: {sort.toUpperCase()}
    </a>
    <a href={viewHref(view === 'list' ? 'grid' : 'list')} class="t-meta">
      VIEW: {view.toUpperCase()}
    </a>
  </div>
</div>

<style>
  .filter-bar { display: flex; justify-content: space-between; align-items: center; padding: 16px 0; border-bottom: 1px solid var(--border-subtle); }
  .chips { display: flex; gap: 12px; }
  .chips a { padding: 6px 12px; border: 1px solid var(--border-default); font-family: var(--font-mono); font-size: 11px; letter-spacing: 0.08em; color: var(--fg-secondary); text-decoration: none; }
  .chips a.on { color: var(--fg-primary); border-color: var(--accent); }
  .controls { display: flex; gap: 16px; }
</style>
```

- [ ] **Step 3: Create `DraftCard.svelte`**

```svelte
<script lang="ts">
  import PhotoTitle from './PhotoTitle.svelte';
  import Button from '../Button.svelte';

  let { photo }: { photo: { id: string; target?: string | null; original_name: string; last_step?: string | null; status: string } } = $props();

  let stepLabel = $derived(
    photo.status === 'processing' ? 'STEP · PROCESSING' :
    photo.status === 'failed' ? 'STEP · FAILED' :
    photo.last_step === 'verify' ? 'STEP 02 · VERIFYING DATA' :
    photo.last_step === 'caption' ? 'STEP 03 · CAPTION & PUBLISH' :
    'STEP 01 · UPLOADED'
  );

  let resumeHref = $derived(
    photo.status === 'failed' ? `/upload/${photo.id}/verify` :
    photo.last_step === 'caption' ? `/upload/${photo.id}/caption` :
    `/upload/${photo.id}/verify`
  );
</script>

<div class="draft-card">
  <div class="thumb">
    {#if photo.status === 'ready'}
      <img src={`/api/photos/${photo.id}/thumb/400`} alt="" />
    {:else}
      <div class="placeholder">{photo.status === 'failed' ? 'FAILED' : 'PROCESSING'}</div>
    {/if}
  </div>
  <div class="t-eyebrow accent">{stepLabel}</div>
  <PhotoTitle {photo} size="sm" />
  <Button variant="secondary" href={resumeHref}>Resume →</Button>
</div>

<style>
  .draft-card { display: flex; flex-direction: column; gap: 8px; padding: 16px; background: var(--bg-surface); border: 1px dashed var(--warning, #c0a060); }
  .thumb { aspect-ratio: 4 / 3; background: var(--bg-canvas); display: flex; align-items: center; justify-content: center; }
  .thumb img { width: 100%; height: 100%; object-fit: cover; }
  .placeholder { font-family: var(--font-mono); font-size: 10px; letter-spacing: 0.12em; color: var(--fg-muted); }
</style>
```

- [ ] **Step 4: Create `DraftsCallout.svelte`**

```svelte
<script lang="ts">
  import DraftCard from './DraftCard.svelte';
  let { drafts }: { drafts: Array<{ id: string; target?: string | null; original_name: string; last_step?: string | null; status: string }> } = $props();
  let displayed = $derived(drafts.slice(0, 3));
</script>

<section class="callout">
  <div class="callout-header">
    <span class="t-eyebrow accent">● {drafts.length} DRAFTS · NOT YET PUBLISHED</span>
    <a href="/account/frames?filter=drafts" class="t-meta">SEE ALL DRAFTS →</a>
  </div>
  <div class="grid">
    {#each displayed as draft (draft.id)}
      <DraftCard photo={draft} />
    {/each}
  </div>
</section>

<style>
  .callout { padding: 24px 64px; background: rgba(208, 160, 80, 0.06); margin: 24px -64px; }
  .callout-header { display: flex; justify-content: space-between; margin-bottom: 16px; }
  .grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 16px; }
  @media (max-width: 900px) { .grid { grid-template-columns: 1fr; } .callout { padding: 24px; margin: 24px 0; } }
</style>
```

- [ ] **Step 5: Create `PhotosTable.svelte`**

```svelte
<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import PhotoTitle from './PhotoTitle.svelte';

  let { rows }: { rows: Array<{
    id: string; target?: string | null; original_name: string;
    taken_at?: string | null; exposure_s?: number | null;
    is_draft: boolean; status: string;
    appreciation_count: number;
  }> } = $props();

  let polling = $state<number | null>(null);
  let needsPolling = $derived(rows.some((r) => r.status === 'processing'));

  $effect(() => {
    if (needsPolling && polling === null) {
      polling = window.setInterval(() => invalidateAll(), 3000);
    }
    if (!needsPolling && polling !== null) {
      clearInterval(polling); polling = null;
    }
    return () => { if (polling !== null) clearInterval(polling); };
  });

  function formatDate(s: string | null | undefined): string {
    if (!s) return '—';
    const d = new Date(s);
    return d.toLocaleDateString('en-GB', { day: '2-digit', month: 'short', year: 'numeric' });
  }
</script>

<table class="photos-table">
  <thead>
    <tr>
      <th></th><th>Target</th><th>Captured</th><th>Integration</th>
      <th>Status</th><th>♡</th><th></th>
    </tr>
  </thead>
  <tbody>
    {#each rows as row (row.id)}
      <tr class:is-draft={row.is_draft}>
        <td class="thumb-cell">
          {#if row.status === 'ready'}
            <img src={`/api/photos/${row.id}/thumb/400`} class="thumb" alt="" />
          {:else}
            <div class="thumb placeholder">{row.status === 'failed' ? 'FAILED' : 'PROCESSING'}</div>
          {/if}
        </td>
        <td><a href="/photo/{row.id}"><PhotoTitle photo={row} size="sm" /></a></td>
        <td>{formatDate(row.taken_at)}</td>
        <td>{row.exposure_s ? `${row.exposure_s} s` : '—'}</td>
        <td>
          {#if row.status === 'processing'}<span class="chip chip-muted">PROCESSING</span>
          {:else if row.status === 'failed'}<span class="chip chip-danger">FAILED</span>
          {:else if row.is_draft}<span class="chip chip-warning">DRAFT</span>
          {:else}<span class="chip chip-accent">PUBLISHED</span>{/if}
        </td>
        <td>{row.is_draft ? '—' : row.appreciation_count}</td>
        <td>⋯</td>
      </tr>
    {/each}
  </tbody>
</table>

<style>
  .photos-table { width: 100%; border-collapse: collapse; }
  .photos-table th, .photos-table td { padding: 12px 8px; text-align: left; border-bottom: 1px solid var(--border-subtle); }
  .photos-table th { font-family: var(--font-mono); font-size: 10px; letter-spacing: 0.12em; color: var(--fg-muted); text-transform: uppercase; }
  .thumb-cell { width: 76px; }
  .thumb { width: 60px; height: 60px; object-fit: cover; }
  .thumb.placeholder { display: flex; align-items: center; justify-content: center; background: var(--bg-surface); font-family: var(--font-mono); font-size: 9px; letter-spacing: 0.1em; color: var(--fg-muted); }
  tr.is-draft { opacity: 0.78; }
  tr.is-draft .thumb { border: 1px dashed var(--warning, #c0a060); position: relative; }
  .chip { padding: 2px 8px; font-family: var(--font-mono); font-size: 10px; letter-spacing: 0.08em; }
  .chip-accent { color: var(--accent); border: 1px solid var(--accent); }
  .chip-warning { color: var(--warning, #c0a060); border: 1px solid var(--warning, #c0a060); }
  .chip-muted { color: var(--fg-muted); border: 1px solid var(--border-default); }
  .chip-danger { color: var(--danger); border: 1px solid var(--danger); }
</style>
```

- [ ] **Step 6: Create `+page.server.ts`**

```ts
import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ locals, url, fetch, cookies }) => {
  if (!locals.user) redirect(303, `/signin?next=${encodeURIComponent(url.pathname + url.search)}`);
  const filter = (url.searchParams.get('filter') ?? 'all') as 'all' | 'published' | 'drafts';
  const sort = (url.searchParams.get('sort') ?? 'newest') as 'newest' | 'oldest';
  const view = (url.searchParams.get('view') ?? 'list') as 'list' | 'grid';
  const cookie = cookies.getAll().map((c) => `${c.name}=${c.value}`).join('; ');

  const [stats, published, drafts] = await Promise.all([
    fetch(`${API}/api/me/stats`, { headers: { Cookie: cookie } }).then((r) => r.json()),
    fetch(`${API}/api/photos?owner_id=${locals.user.id}`, { headers: { Cookie: cookie } }).then((r) => r.json()),
    fetch(`${API}/api/photos?drafts=true`, { headers: { Cookie: cookie } }).then((r) => r.json())
  ]);

  let rows: Array<unknown> =
    filter === 'drafts' ? drafts.photos :
    filter === 'published' ? published.photos :
    [...drafts.photos, ...published.photos];
  if (sort === 'oldest') rows = [...rows].reverse();

  return {
    stats, filter, sort, view, rows,
    drafts: drafts.photos,
    counts: {
      all: published.photos.length + drafts.photos.length,
      published: published.photos.length,
      drafts: drafts.photos.length
    }
  };
};
```

- [ ] **Step 7: Create `+page.svelte`**

```svelte
<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import StatsRow from '$lib/components/photos/StatsRow.svelte';
  import DraftsCallout from '$lib/components/photos/DraftsCallout.svelte';
  import FilterChips from '$lib/components/photos/FilterChips.svelte';
  import PhotosTable from '$lib/components/photos/PhotosTable.svelte';
  import Button from '$lib/components/Button.svelte';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  let isEmpty = $derived(data.counts.all === 0);
</script>

<svelte:head><title>My frames — Astrophoto</title></svelte:head>
<AppHeader active="Profile" />

<div class="frames-page">
  {#if isEmpty}
    <div class="empty">
      <h1>An empty plate, waiting for first light.</h1>
      <Button variant="primary" href="/upload" size="lg">Upload a frame</Button>
    </div>
  {:else}
    <header class="title-row">
      <h1>My frames</h1>
      <StatsRow stats={data.stats} />
    </header>

    {#if data.drafts.length > 0 && data.filter !== 'drafts'}
      <DraftsCallout drafts={data.drafts} />
    {/if}

    <FilterChips active={data.filter} counts={data.counts} sort={data.sort} view={data.view} />

    {#if data.filter === 'drafts' && data.drafts.length === 0}
      <p class="empty-msg">No drafts. Every frame you upload is published.
        <a href="/upload">Upload a frame</a></p>
    {:else}
      <PhotosTable rows={data.rows} />
    {/if}
  {/if}
</div>

<style>
  .frames-page { padding: 40px 64px; max-width: 1280px; margin: 0 auto; }
  .title-row { display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 32px; }
  .title-row h1 { font-family: var(--font-display); font-size: 44px; margin: 0; }
  .empty { text-align: center; padding: 120px 24px; }
  .empty h1 { font-family: var(--font-display); font-size: 32px; margin-bottom: 24px; }
  .empty-msg { padding: 40px 0; color: var(--fg-secondary); }
  @media (max-width: 768px) { .frames-page { padding: 32px 24px; } .title-row { flex-direction: column; gap: 16px; align-items: flex-start; } }
</style>
```

- [ ] **Step 8: Create `account/frames/drafts/+page.server.ts`**

```ts
import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async () => {
  redirect(303, '/account/frames?filter=drafts');
};
```

- [ ] **Step 9: Manual smoke + commit**

```bash
just dev
# Sign in, upload a draft, browse to /account/frames, see stats + drafts callout + table.
git add frontend/src/routes/account/ frontend/src/lib/components/photos/
git commit -m "feat(frontend): /account/frames dashboard with stats, drafts callout, table + 5 new components"
```

---

## Task 19: Frontend — photo detail draft strip + ⋯ menu + REPROCESSED + ReplaceModal

**Files:**
- Create: `frontend/src/lib/components/photos/ReplaceModal.svelte`
- Modify: `frontend/src/routes/photo/[slug]/+page.server.ts` — pass `is_draft`, `last_step`, `replaced_at`, `original_uploaded_at`, `current_user_id`.
- Modify: `frontend/src/routes/photo/[slug]/+page.svelte` — render the DRAFT strip, ⋯ owner menu, REPROCESSED line, mount ReplaceModal.

- [ ] **Step 1: Create `ReplaceModal.svelte`**

```svelte
<script lang="ts">
  import Modal from '../Modal.svelte';
  import Button from '../Button.svelte';

  let { open = $bindable(false), photoId, onreplaced }: {
    open: boolean; photoId: string; onreplaced: () => void;
  } = $props();

  let file = $state<File | null>(null);
  let busy = $state(false);
  let err = $state<string | null>(null);

  async function submit() {
    if (!file) { err = 'Choose a file.'; return; }
    busy = true; err = null;
    const fd = new FormData();
    fd.append('file', file, file.name);
    const r = await fetch(`/api/photos/${photoId}/replace`, {
      method: 'POST', body: fd, credentials: 'include'
    });
    busy = false;
    if (!r.ok) { err = `Replace failed: ${await r.text()}`; return; }
    open = false; file = null;
    onreplaced();
  }
</script>

<Modal bind:open title="Replace image">
  <p class="t-meta">Caption, target, comments and appreciations are kept. Pipeline reprocesses thumbnails.</p>
  <input type="file" accept="image/jpeg,image/png,image/tiff"
    onchange={(e) => (file = (e.target as HTMLInputElement).files?.[0] ?? null)} />
  {#if err}<p class="t-meta form-error">{err}</p>{/if}
  <div slot="actions">
    <Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
    <Button variant="primary" disabled={!file || busy} onclick={submit}>
      {busy ? 'Uploading…' : 'Replace'}
    </Button>
  </div>
</Modal>

<style>
  .form-error { color: var(--danger); }
</style>
```

(If `Modal.svelte` doesn't expose a slot named `actions`, adapt the call. Check `frontend/src/lib/components/Modal.svelte` for its actual API and align.)

- [ ] **Step 2: Extend `+page.server.ts`**

In `frontend/src/routes/photo/[slug]/+page.server.ts`, ensure the loader passes the new fields and `current_user_id`:

```ts
return {
  photo, // already includes is_draft, last_step, replaced_at, original_uploaded_at
  current_user_id: locals.user?.id ?? null
};
```

- [ ] **Step 3: Extend `+page.svelte`**

Add owner-only chrome: a draft strip directly under `<AppHeader>`, the ⋯ action menu button on the photo column, the REPROCESSED line in the sidebar, and mount the ReplaceModal:

```svelte
<script lang="ts">
  import ReplaceModal from '$lib/components/photos/ReplaceModal.svelte';
  import { invalidateAll } from '$app/navigation';
  // … existing imports …
  let isOwner = $derived(data.current_user_id && data.photo.owner_id === data.current_user_id);
  let replaceOpen = $state(false);
  let menuOpen = $state(false);

  function continueHref() {
    if (!data.photo.last_step || data.photo.last_step === 'upload' || data.photo.last_step === 'verify')
      return `/upload/${data.photo.id}/verify`;
    return `/upload/${data.photo.id}/caption`;
  }

  async function discard() {
    if (!confirm('Discard this draft? This cannot be undone.')) return;
    await fetch(`/api/photos/${data.photo.id}`, { method: 'DELETE', credentials: 'include' });
    location.href = '/account/frames';
  }

  function formatRange(a: string, b: string): string {
    const da = new Date(a), db = new Date(b);
    const sameYear = da.getFullYear() === db.getFullYear();
    const fmtShort = { day: '2-digit', month: 'short' } as const;
    const fmtLong = { day: '2-digit', month: 'short', year: 'numeric' } as const;
    return `${da.toLocaleDateString('en-GB', fmtShort).toUpperCase()} → ${db.toLocaleDateString('en-GB', sameYear ? fmtShort : fmtLong).toUpperCase()}`;
  }
</script>

{#if isOwner && data.photo.is_draft}
  <div class="draft-strip">
    <span class="t-eyebrow accent">● DRAFT · ONLY YOU CAN SEE THIS</span>
    <div class="strip-actions">
      <a href={continueHref()} class="btn btn-secondary btn-sm">Continue editing →</a>
      <button class="btn btn-ghost btn-sm" onclick={discard}>Discard</button>
    </div>
  </div>
{/if}

<!-- Existing photo column JSX, plus an owner-only action button: -->
{#if isOwner}
  <div class="action-menu">
    <button class="btn btn-ghost btn-sm" onclick={() => (menuOpen = !menuOpen)} aria-label="Actions">⋯</button>
    {#if menuOpen}
      <ul class="menu-popover">
        <li><a href="/upload/{data.photo.id}/verify">Edit metadata</a></li>
        <li><button onclick={() => { replaceOpen = true; menuOpen = false; }}>Replace image…</button></li>
        <li>{#if data.photo.is_draft}<button onclick={discard}>Discard draft</button>{:else}<button onclick={discard}>Delete photo</button>{/if}</li>
      </ul>
    {/if}
  </div>
{/if}

<!-- Sidebar additions: REPROCESSED line directly under the published-date eyebrow -->
{#if data.photo.replaced_at}
  <div class="t-eyebrow muted">● REPROCESSED · {formatRange(data.photo.original_uploaded_at, data.photo.replaced_at)}</div>
{/if}

<ReplaceModal bind:open={replaceOpen} photoId={data.photo.id} onreplaced={() => invalidateAll()} />

<style>
  .draft-strip { display: flex; justify-content: space-between; align-items: center; padding: 12px 64px; background: rgba(208, 160, 80, 0.08); border-bottom: 1px solid var(--warning, #c0a060); }
  .strip-actions { display: flex; gap: 8px; }
  .action-menu { position: relative; display: inline-block; }
  .menu-popover { position: absolute; top: 100%; right: 0; background: var(--bg-surface); border: 1px solid var(--border-default); list-style: none; padding: 4px 0; min-width: 180px; z-index: 10; }
  .menu-popover li > a, .menu-popover li > button { display: block; width: 100%; text-align: left; padding: 8px 16px; background: none; border: none; color: var(--fg-primary); cursor: pointer; }
  .menu-popover li > a:hover, .menu-popover li > button:hover { background: var(--bg-canvas); }
  .muted { color: var(--fg-muted); }
</style>
```

- [ ] **Step 4: Manual smoke + commit**

```bash
just dev
# Verify: own draft → strip shown; replace flow → Modal open → upload → REPROCESSED appears.
git add frontend/src/lib/components/photos/ReplaceModal.svelte frontend/src/routes/photo/\[slug\]/
git commit -m "feat(photo-detail): owner DRAFT strip, ⋯ action menu, REPROCESSED line, ReplaceModal"
```

---

## Task 20: Polish 8.5 — Context-aware home eyebrow + FollowButton 3-state

**Files:**
- Modify: `frontend/src/routes/+page.server.ts` — load `following_count` for authed users.
- Modify: `frontend/src/routes/+page.svelte` — branch the eyebrow on `following_count`.
- Modify: `frontend/src/lib/components/FollowButton.svelte` — 3 visual states with hover transition.

- [ ] **Step 1: Home eyebrow**

In `frontend/src/routes/+page.server.ts` add to the load return:

```ts
return {
  // … existing fields …
  following_count: locals.user?.following_ids?.length ?? 0
};
```

In `frontend/src/routes/+page.svelte`, replace the existing eyebrow render with:

```svelte
{#if data.user && data.following_count > 0}
  <span class="t-eyebrow accent">● FROM THE {data.following_count} PHOTOGRAPHERS YOU FOLLOW</span>
{:else}
  <span class="t-eyebrow">● {dateString} · {weekday}</span>
{/if}
```

(`dateString` and `weekday` are already computed at the top of the existing template; if not, derive them inline with `new Date()`.)

- [ ] **Step 2: FollowButton 3-state**

Replace `frontend/src/lib/components/FollowButton.svelte` body. The button has three visual states: not following, following · default (accent), following · hover (danger with "Unfollow?" copy). All transitions are CSS-only:

```svelte
<script lang="ts">
  let { following = false, onclick }: {
    following: boolean; onclick: () => Promise<void> | void;
  } = $props();

  let busy = $state(false);
  async function handle() {
    if (busy) return;
    busy = true;
    try { await onclick(); } finally { busy = false; }
  }
</script>

<button
  type="button"
  class="follow-btn"
  class:following
  class:busy
  disabled={busy}
  onclick={handle}
>
  {#if following}
    <span class="label-default">✓ Following</span>
    <span class="label-hover">Unfollow?</span>
  {:else}
    <span>Follow</span>
  {/if}
</button>

<style>
  .follow-btn {
    padding: 6px 14px; font-family: var(--font-mono); font-size: 12px;
    background: transparent; cursor: pointer; transition: all 200ms;
    border: 1px solid var(--accent); color: var(--accent);
  }
  .follow-btn:not(.following) {
    background: var(--accent); color: var(--bg-canvas);
  }
  .follow-btn.following .label-hover { display: none; }
  .follow-btn.following:hover { border-color: var(--danger); color: var(--danger); }
  .follow-btn.following:hover .label-default { display: none; }
  .follow-btn.following:hover .label-hover { display: inline; }
  .follow-btn.busy { opacity: 0.6; cursor: wait; }
</style>
```

(Update existing call sites — `frontend/src/routes/u/[username]/+page.svelte` etc. — to pass `following={...}` and `onclick={...}` matching the new prop shape. If the previous prop shape used different names, adapt callers.)

- [ ] **Step 3: Commit**

```bash
git add frontend/src/routes/+page.server.ts frontend/src/routes/+page.svelte frontend/src/lib/components/FollowButton.svelte frontend/src/routes/u/
git commit -m "feat(polish 8.5): context-aware home eyebrow + FollowButton 3-state hover"
```

---

## Task 21: Polish 8.5 — Mobile sticky AppreciateButton

**Files:**
- Modify: `frontend/src/lib/components/AppreciateButton.svelte` — add `variant: 'inline' | 'mobile-sticky'` prop.
- Modify: `frontend/src/routes/photo/[slug]/+page.svelte` — render second instance with `variant="mobile-sticky"`, hidden above 640 px.

- [ ] **Step 1: Add the variant**

In `frontend/src/lib/components/AppreciateButton.svelte`, add a `variant` prop and split the styles. Mobile variant renders a 64 px tall fixed bar at the bottom of the viewport with a heart pill, comment pill, and share icon.

```svelte
<script lang="ts">
  let { photoId, count, appreciated, variant = 'inline', onToggle }: {
    photoId: string; count: number; appreciated: boolean;
    variant?: 'inline' | 'mobile-sticky';
    onToggle: () => Promise<void>;
  } = $props();
  // … keep existing toggle logic …
</script>

{#if variant === 'inline'}
  <button onclick={onToggle} class="inline-btn" class:active={appreciated}>
    {appreciated ? '♥' : '♡'} {count}
  </button>
{:else}
  <div class="mobile-sticky">
    <button onclick={onToggle} class="pill" class:active={appreciated}>
      {appreciated ? '♥' : '♡'} <span class="num">{count}</span>
    </button>
    <a href="#comments" class="pill">💬</a>
    <button class="pill" onclick={() => navigator.share?.({ url: location.href })}>↗</button>
  </div>
{/if}

<style>
  .inline-btn { /* existing inline styles */ }
  .mobile-sticky {
    position: fixed; bottom: 0; left: 0; right: 0; height: 64px;
    background: var(--bg-overlay, rgba(20,20,20,0.85));
    backdrop-filter: blur(12px); border-top: 1px solid var(--border-subtle);
    padding-bottom: env(safe-area-inset-bottom);
    display: flex; gap: 12px; align-items: center; justify-content: space-around;
    z-index: 100;
  }
  .pill { height: 44px; padding: 0 16px; border: 1px solid var(--border-default); background: transparent; color: var(--fg-primary); display: inline-flex; align-items: center; gap: 8px; border-radius: 22px; font-family: var(--font-mono); font-size: 14px; cursor: pointer; }
  .pill.active { background: rgba(208,160,80,0.12); border-color: var(--accent); color: var(--accent); }
</style>
```

- [ ] **Step 2: Mount the mobile variant**

In `frontend/src/routes/photo/[slug]/+page.svelte`, add at the bottom of the template:

```svelte
<div class="mobile-only">
  <AppreciateButton variant="mobile-sticky"
    photoId={data.photo.id} count={data.photo.appreciation_count}
    appreciated={appreciatedState} onToggle={toggleAppreciate} />
</div>

<style>
  .mobile-only { display: none; }
  @media (max-width: 640px) { .mobile-only { display: block; } }
</style>
```

(Use whatever local state/handler the existing inline button uses for `appreciated` and `toggleAppreciate`.)

- [ ] **Step 3: Commit**

```bash
git add frontend/src/lib/components/AppreciateButton.svelte frontend/src/routes/photo/\[slug\]/
git commit -m "feat(polish 8.5): mobile sticky AppreciateButton bar"
```

---

## Task 22: Use `PhotoTitle` everywhere with target fallback

**Files:**
- Modify each surface to render `<PhotoTitle photo={p} />` instead of bespoke target/name rendering:
  - `frontend/src/routes/photo/[slug]/+page.svelte` (header, size=lg)
  - `frontend/src/routes/u/[username]/+page.svelte` (cards, size=md)
  - `frontend/src/routes/+page.svelte` (gallery cards, size=md)
  - PhotosTable + DraftCard already use it.

Mechanical replacement: find any `{photo.target ?? photo.original_name}` or similar inline expressions and replace with `<PhotoTitle {photo} size="lg" />` (or appropriate size).

- [ ] **Step 1: grep, replace, manual visual check**

```bash
cd frontend
grep -rn "photo.target ?? photo.original_name\|target ?? original_name\|target || original_name" src/routes/
# Replace each occurrence with <PhotoTitle … />.
just dev
# Browse: home, profile page, photo detail. Confirm UNTITLED chip appears for photos without target.
```

- [ ] **Step 2: Commit**

```bash
git add frontend/src/routes/
git commit -m "feat(polish 8.5): PhotoTitle adopted across home/profile/detail surfaces"
```

---

## Task 23: e2e Playwright suite + final `just check`

**Files:**
- Create: `frontend/tests/e2e/photos_phase8b.spec.ts`

- [ ] **Step 1: Write the spec**

```ts
import { expect, test } from '@playwright/test';
import { signupAndLogin, uploadJpeg } from './helpers';

test('upload a draft, find it in /account/frames, publish from caption step', async ({ page }) => {
  await signupAndLogin(page, 'alice');
  await page.goto('/upload');
  await uploadJpeg(page, 'fixtures/sample.jpg');
  await expect(page).toHaveURL(/\/upload\/.+\/verify/);
  await page.getByLabel('TARGET').fill('M31');
  await page.getByRole('button', { name: 'Save as draft' }).click();
  await expect(page).toHaveURL('/account/frames');
  await expect(page.getByText('● 1 DRAFTS · NOT YET PUBLISHED')).toBeVisible();

  await page.getByRole('link', { name: 'Resume →' }).first().click();
  await page.getByRole('button', { name: 'Continue →' }).click();
  await page.getByRole('textbox', { name: 'Describe' }).fill('first light');
  await page.getByRole('button', { name: 'Publish' }).click();
  await expect(page).toHaveURL(/\/photo\/.+/);
  await expect(page.getByText('DRAFT · ONLY YOU CAN SEE THIS')).toHaveCount(0);
});

test('edit metadata of a published photo via ⋯ menu, save changes, no republish', async ({ page }) => {
  await signupAndLogin(page, 'alice');
  // … upload + publish helper omitted for brevity; reuse from upload flow above.
});

test('replace a published photo, REPROCESSED label appears on detail', async ({ page }) => {
  // … upload, publish, click ⋯ → Replace image…, drop a new file, confirm.
  // Assert a "REPROCESSED" line appears in the sidebar.
});

test('untitled photo on home shows UNTITLED chip', async ({ page }) => {
  // Upload without target, publish, browse home, see UNTITLED chip.
});

test('mobile viewport: sticky AppreciateButton bar appears on detail', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 812 });
  // … browse to a published photo, expect the .mobile-sticky bar to be visible.
});

test('FollowButton toggles through 3 states with correct copy', async ({ page }) => {
  // … on a profile page, hover the Following state, expect "Unfollow?" copy.
});
```

(Reuse `frontend/tests/e2e/helpers.ts` if it exists; otherwise create signup + JPEG upload helpers.)

- [ ] **Step 2: Run the suite**

```bash
just dev   # in another terminal
cd frontend && pnpm test:e2e photos_phase8b
```
Expected: all 6 specs pass. Fix any selector / wait races inline.

- [ ] **Step 3: Final quality gates**

```bash
just check
```
Expected: zero output. Fix any clippy / type-check / fmt issues.

- [ ] **Step 4: Commit**

```bash
git add frontend/tests/e2e/photos_phase8b.spec.ts
git commit -m "test(e2e): Phase 8b drafts/replace/polish coverage"
```

---

## Definition of done

- `just check` passes with zero output.
- `just test` (backend integration + frontend e2e) green.
- All Phase 8b spec sections implemented:
  - Migration 0004 applied (drafts, replace, pipeline_error, photo_pending_deletes).
  - `is_visible_to` gates every public per-photo endpoint.
  - `POST /:id/publish`, `PUT /:id`, `POST /:id/replace`, `GET /api/me/stats` shipped.
  - Pipeline supports `Replace` mode with deferred S3 deletion drained on success.
  - Hourly worker sweeps stale `photo_pending_deletes`.
  - `/upload` is a 3-step flow (`/`, `/[id]/verify`, `/[id]/caption`).
  - `/account/frames` dashboard with stats / drafts callout / table.
  - Photo detail shows DRAFT strip (owner), ⋯ action menu, REPROCESSED line, ReplaceModal.
  - Polish 8.5: context-aware eyebrow, FollowButton 3-state, AppreciateButton mobile-sticky, PhotoTitle untitled fallback adopted.
- Hand off to `superpowers:finishing-a-development-branch`.
