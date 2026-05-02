# Phase 6 Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the visible UX gaps from Phase 5: drop the hardcoded "SHO" tag, render an EXIF panel that adapts to whatever fields a real photo has, expose `GET /api/users/:id` so usernames resolve on profile pages, dedup hero/grid, add an avatar dropdown with Sign out, and ship a `just seed` command that imports a bundled NASA fixture.

**Architecture:** Backend gets a public users endpoint and a refactor that extracts the photo processing pipeline into a reusable `photos::pipeline::process` callable from both the HTTP handler and a new seed binary. Frontend gets an `AvatarMenu` component that owns logout, an EXIF row builder that filters null fields, and a hero/grid split.

**Tech Stack:** axum 0.7, sqlx 0.8, ts-rs (Rust → TS codegen), Svelte 5 runes, tower (no new deps).

**Spec reference:** `docs/superpowers/specs/2026-05-02-phase-6-polish-design.md`

**Working directory for all commands:** `/Volumes/Pascal4Tb/Projects/astrophoto/` (referred to below as `$ROOT`).

**Branch:** `feat/phase-6-polish` (already created with the spec committed).

---

## Task 1: Add `count_by_owner` query to `photos::queries`

**Files:**
- Modify: `backend/src/photos/queries.rs` (append a new function)

- [ ] **Step 1: Add the function**

Append at the end of `backend/src/photos/queries.rs` (just before any existing `#[cfg(test)]`):

```rust
pub async fn count_by_owner(pool: &PgPool, owner_id: Uuid) -> Result<i64, AppError> {
    let row = sqlx::query!(
        r#"select count(*) as "count!" from photos where owner_id = $1 and status = 'ready'"#,
        owner_id
    )
    .fetch_one(pool)
    .await?;
    Ok(row.count)
}
```

The `count(*)` cast is necessary because Postgres returns `bigint` (i64), and `count(*)` is nullable in some sqlx versions — the `as "count!"` annotation forces the non-null view.

- [ ] **Step 2: Refresh the sqlx offline cache**

Run from `$ROOT`:

```bash
cd backend
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto cargo sqlx prepare -- --lib
```

Expected: a new `.sqlx/query-*.json` file appears.

- [ ] **Step 3: Verify compile**

Run: `cd $ROOT/backend && cargo check`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
cd $ROOT
git add backend/src/photos/queries.rs backend/.sqlx/
git commit -m "feat(backend/photos): add count_by_owner query"
```

---

## Task 2: Create `users::get` handler with `UserPublic` DTO

**Files:**
- Create: `backend/src/users/get.rs`
- Modify: `backend/src/users/mod.rs` (add `pub mod get;`)
- Modify: `backend/src/api_types.rs` (add `UserPublic` DTO)
- Modify: `backend/src/bin/gen-types.rs` (export `UserPublic`)

- [ ] **Step 1: Add `UserPublic` to `api_types.rs`**

Append to `backend/src/api_types.rs` (after the existing `AuthError` type):

```rust
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "UserPublic.ts")]
pub struct UserPublic {
    pub id: String,
    pub display_name: String,
    pub created_at: String,
    pub photo_count: i64,
}
```

- [ ] **Step 2: Create `backend/src/users/get.rs`**

```rust
use axum::{Json, extract::{Path, State}};
use uuid::Uuid;

use crate::api_types::UserPublic;
use crate::http::AppState;
use crate::photos::queries as photo_q;
use crate::users::queries as user_q;
use crate::AppError;

pub async fn handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserPublic>, AppError> {
    let user = user_q::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound)?;
    let count = photo_q::count_by_owner(&state.pool, id).await?;
    Ok(Json(UserPublic {
        id: user.id.to_string(),
        display_name: user.display_name,
        created_at: user.created_at.to_rfc3339(),
        photo_count: count,
    }))
}
```

- [ ] **Step 3: Wire the module**

Modify `backend/src/users/mod.rs` to include:

```rust
pub mod get;
pub mod queries;

use crate::api_types::User;
use queries::UserRow;

impl From<UserRow> for User {
    // ... existing impl unchanged
```

(Only the first line is new — `pub mod get;`. Keep everything else as-is.)

- [ ] **Step 4: Update `bin/gen-types.rs` to export `UserPublic`**

Modify `backend/src/bin/gen-types.rs`:

```rust
use std::fs;
use std::path::Path;
use ts_rs::TS;
use astrophoto::api_types::{Health, User, AuthError, UserPublic};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = Path::new("../frontend/src/lib/api");
    fs::create_dir_all(out_dir)?;
    Health::export_all_to(out_dir)?;
    User::export_all_to(out_dir)?;
    AuthError::export_all_to(out_dir)?;
    UserPublic::export_all_to(out_dir)?;
    println!("Wrote types to: {}", out_dir.display());
    Ok(())
}
```

- [ ] **Step 5: Verify compile**

Run: `cd $ROOT/backend && cargo check`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
cd $ROOT
git add backend/src/users/get.rs backend/src/users/mod.rs backend/src/api_types.rs backend/src/bin/gen-types.rs
git commit -m "feat(backend/users): add UserPublic DTO and GET handler scaffolding"
```

---

## Task 3: Mount `GET /api/users/:id` route

**Files:**
- Modify: `backend/src/http/mod.rs`

- [ ] **Step 1: Add the route**

In `backend/src/http/mod.rs`, find the `pub fn router(...)` function. Add a new `.route(...)` line in the chain. The exact position depends on existing layout, but place it next to the photos routes for locality. The new line is:

```rust
.route("/api/users/:id", axum::routing::get(crate::users::get::handler))
```

- [ ] **Step 2: Verify compile**

Run: `cd $ROOT/backend && cargo check`
Expected: clean.

- [ ] **Step 3: Smoke test live**

Backend must be running (`cargo run --bin astrophoto` with the standard env vars; postgres on :5434). In another terminal:

```bash
# Sign up to get a user id
EMAIL="users-endpoint-$(date +%s)@test.com"
SIGNUP=$(curl -i -s -X POST http://localhost:8080/api/auth/signup \
  -H 'Content-Type: application/json' \
  -d "{\"email\":\"$EMAIL\",\"password\":\"longenoughpw\",\"display_name\":\"Endpoint Tester\"}")
USER_ID=$(echo "$SIGNUP" | grep -oE '"id":"[^"]+"' | head -1 | cut -d'"' -f4)
echo "user_id=$USER_ID"

# Hit the new endpoint
curl -s "http://localhost:8080/api/users/$USER_ID"
```

Expected: `{"id":"<uuid>","display_name":"Endpoint Tester","created_at":"2026-05-...","photo_count":0}`. NO `email` field in the response.

- [ ] **Step 4: Commit**

```bash
cd $ROOT
git add backend/src/http/mod.rs
git commit -m "feat(backend/users): mount GET /api/users/:id"
```

---

## Task 4: Extract `photos::pipeline::process` from `upload.rs`

**Files:**
- Create: `backend/src/photos/pipeline.rs`
- Modify: `backend/src/photos/mod.rs` (add `pub mod pipeline;`)
- Modify: `backend/src/photos/upload.rs` (replace inline `process_photo` with a call to `pipeline::process`)

- [ ] **Step 1: Create `backend/src/photos/pipeline.rs`**

```rust
//! Photo processing pipeline. Used by both the HTTP upload handler and
//! the seed binary. Synchronous (awaits each step). Caller decides
//! whether to wrap in tokio::spawn for background processing.

use std::sync::Arc;

use bytes::Bytes;
use sqlx::PgPool;
use uuid::Uuid;

use crate::AppError;
use crate::photos::{exif, queries, thumbs};
use crate::storage::Storage;

const THUMB_SIZES: &[u32] = &[400, 1200];

/// Insert + upload original + run EXIF + generate thumbs + mark ready.
/// Returns the photo id on success. On failure marks the row 'failed'
/// (only if we got far enough to insert the row) and propagates the error.
pub async fn process(
    pool: &PgPool,
    storage: Arc<dyn Storage>,
    owner_id: Uuid,
    original_name: &str,
    mime: &str,
    target: Option<&str>,
    caption: Option<&str>,
    bytes: Bytes,
) -> Result<Uuid, AppError> {
    let photo_id = Uuid::new_v4();
    let storage_key = format!("originals/{photo_id}");
    storage.put(&storage_key, mime, bytes.clone()).await?;
    queries::insert_processing(
        pool,
        owner_id,
        &storage_key,
        original_name,
        bytes.len() as i64,
        mime,
        target,
        caption,
    )
    .await?;

    if let Err(e) = process_inner(pool, storage, photo_id, bytes).await {
        let _ = queries::mark_failed(pool, photo_id).await;
        return Err(e);
    }
    Ok(photo_id)
}

async fn process_inner(
    pool: &PgPool,
    storage: Arc<dyn Storage>,
    photo_id: Uuid,
    bytes: Bytes,
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

    queries::mark_ready(pool, photo_id, full_w, full_h, &exif_data).await?;
    Ok(())
}
```

- [ ] **Step 2: Wire the module**

In `backend/src/photos/mod.rs`, add `pub mod pipeline;` next to the other modules.

- [ ] **Step 3: Refactor `upload.rs` to use `pipeline::process`**

Replace the body of `backend/src/photos/upload.rs::handler` and remove the local `process_photo` function. The new handler:

```rust
use axum::{
    Json,
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
};
use bytes::Bytes;
use serde::Serialize;

use crate::auth::middleware::CurrentUser;
use crate::http::AppState;
use crate::photos::pipeline;
use crate::AppError;

const MAX_BYTES: usize = 50 * 1024 * 1024; // 50 MB
const ALLOWED_MIMES: &[&str] = &["image/jpeg", "image/png", "image/tiff"];

#[derive(Serialize)]
pub struct UploadResponse {
    pub id: String,
    pub status: String,
}

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let mut file_bytes: Option<Bytes> = None;
    let mut filename = String::from("upload.bin");
    let mut mime = String::from("application/octet-stream");
    let mut target: Option<String> = None;
    let mut caption: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::Validation(format!("multipart: {e}")))?
    {
        match field.name() {
            Some("file") => {
                if let Some(name) = field.file_name() {
                    filename = name.to_string();
                }
                if let Some(ct) = field.content_type() {
                    mime = ct.to_string();
                }
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| AppError::Validation(format!("read: {e}")))?;
                if data.len() > MAX_BYTES {
                    return Err(AppError::Validation(format!(
                        "file too large: {} bytes (max {MAX_BYTES})",
                        data.len()
                    )));
                }
                file_bytes = Some(data);
            }
            Some("target") => {
                target = field.text().await.ok().filter(|s| !s.is_empty());
            }
            Some("caption") => {
                caption = field.text().await.ok().filter(|s| !s.is_empty());
            }
            _ => {}
        }
    }

    let bytes = file_bytes.ok_or_else(|| AppError::Validation("missing file".into()))?;
    if !ALLOWED_MIMES.contains(&mime.as_str()) {
        return Err(AppError::Validation(format!("unsupported mime: {mime}")));
    }

    // Spawn the pipeline in the background so the upload returns 202 fast.
    let pool = state.pool.clone();
    let storage = state.storage.clone();
    let user_id = user.id;
    let filename_owned = filename.clone();
    let mime_owned = mime.clone();
    let target_owned = target.clone();
    let caption_owned = caption.clone();
    let bytes_clone = bytes.clone();

    // We need the photo_id in the response, so do the synchronous insert on
    // the request task and let the heavy work continue in the background.
    let id = pipeline_quickstart(
        &state.pool,
        &state.storage,
        user_id,
        &filename,
        &mime,
        target.as_deref(),
        caption.as_deref(),
        bytes,
    )
    .await?;

    tokio::spawn(async move {
        let _ = pipeline_finalize(&pool, storage, id, bytes_clone).await;
    });

    let _ = (filename_owned, mime_owned, target_owned, caption_owned);

    Ok((
        StatusCode::ACCEPTED,
        Json(UploadResponse {
            id: id.to_string(),
            status: "processing".into(),
        }),
    ))
}

// Internal split: the request task does the upload + DB insert and returns
// the id; the background task does EXIF + thumbnails. Both live in
// pipeline.rs but we expose two helpers for the HTTP path.
use std::sync::Arc;
use uuid::Uuid;

use crate::photos::queries;
use crate::storage::Storage;

async fn pipeline_quickstart(
    pool: &sqlx::PgPool,
    storage: &Arc<dyn Storage>,
    owner_id: Uuid,
    original_name: &str,
    mime: &str,
    target: Option<&str>,
    caption: Option<&str>,
    bytes: Bytes,
) -> Result<Uuid, AppError> {
    let photo_id = Uuid::new_v4();
    let storage_key = format!("originals/{photo_id}");
    storage.put(&storage_key, mime, bytes.clone()).await?;
    queries::insert_processing(
        pool,
        owner_id,
        &storage_key,
        original_name,
        bytes.len() as i64,
        mime,
        target,
        caption,
    )
    .await?;
    Ok(photo_id)
}

async fn pipeline_finalize(
    pool: &sqlx::PgPool,
    storage: Arc<dyn Storage>,
    photo_id: Uuid,
    bytes: Bytes,
) -> Result<(), AppError> {
    if let Err(e) = pipeline::finalize(pool, storage, photo_id, bytes).await {
        tracing::error!(photo_id=%photo_id, error=%e, "photo processing failed");
        let _ = queries::mark_failed(pool, photo_id).await;
        return Err(e);
    }
    Ok(())
}
```

> Note: this restructure introduces a small split — `pipeline::process`
> is the synchronous "do everything and await" form (used by seed),
> while the HTTP handler uses two helpers (`pipeline_quickstart` for
> the synchronous insert returning the id, then `pipeline::finalize`
> in a spawn for thumbnails). This keeps the HTTP path's 202-fast
> behavior while letting the seed run synchronously.

Add a `pub async fn finalize(...)` to `pipeline.rs` exposing the inner
processing step. Update `pipeline.rs` to expose both:

```rust
// In pipeline.rs, change `async fn process_inner` to `pub async fn finalize`
// and have `process` call `finalize`.

pub async fn process(
    pool: &PgPool,
    storage: Arc<dyn Storage>,
    owner_id: Uuid,
    original_name: &str,
    mime: &str,
    target: Option<&str>,
    caption: Option<&str>,
    bytes: Bytes,
) -> Result<Uuid, AppError> {
    let photo_id = Uuid::new_v4();
    let storage_key = format!("originals/{photo_id}");
    storage.put(&storage_key, mime, bytes.clone()).await?;
    queries::insert_processing(
        pool, owner_id, &storage_key, original_name,
        bytes.len() as i64, mime, target, caption,
    ).await?;
    if let Err(e) = finalize(pool, storage, photo_id, bytes).await {
        let _ = queries::mark_failed(pool, photo_id).await;
        return Err(e);
    }
    Ok(photo_id)
}

pub async fn finalize(
    pool: &PgPool,
    storage: Arc<dyn Storage>,
    photo_id: Uuid,
    bytes: Bytes,
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
    let (full_w, full_h) = generated.iter().max_by_key(|t| t.size)
        .map(|t| (t.width as i32, t.height as i32))
        .unwrap_or((0, 0));

    for thumb in generated {
        let key = format!("thumbs/{photo_id}/{}", thumb.size);
        let len = thumb.bytes.len() as i64;
        storage.put(&key, "image/jpeg", thumb.bytes).await?;
        queries::insert_thumbnail(pool, photo_id, thumb.size as i32, &key, len).await?;
    }
    queries::mark_ready(pool, photo_id, full_w, full_h, &exif_data).await?;
    Ok(())
}
```

- [ ] **Step 4: Verify compile and tests still pass**

```bash
cd $ROOT/backend
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto cargo check --tests
cargo test --test photos
```

Expected: photos integration test (full upload pipeline) still passes.

- [ ] **Step 5: Commit**

```bash
cd $ROOT
git add backend/src/photos/pipeline.rs backend/src/photos/mod.rs backend/src/photos/upload.rs
git commit -m "refactor(backend/photos): extract pipeline::process for reuse by seed"
```

---

## Task 5: Add the seed binary

**Files:**
- Create: `backend/src/bin/seed.rs`
- Create: `backend/seeds/fixtures/.gitkeep` (so the directory exists in fresh clones even before fixtures are added)
- Create: `backend/seeds/fixtures/LICENSE.txt` (placeholder; real license added with the actual fixture in Task 6)
- Modify: `backend/Cargo.toml` (declare the new binary target)
- Modify: `justfile` (add `just seed` recipe)

- [ ] **Step 1: Declare the binary in Cargo.toml**

Add to `backend/Cargo.toml` after the existing `[[bin]]` declarations:

```toml
[[bin]]
name = "seed"
path = "src/bin/seed.rs"
```

- [ ] **Step 2: Create `backend/src/bin/seed.rs`**

```rust
//! Demo seed: creates `demo@astrophoto.example` (password `demoaccount`)
//! and uploads any JPEG/PNG files found in `backend/seeds/fixtures/` via
//! the same pipeline as the HTTP upload handler. Idempotent.
//!
//! Run via `just seed`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use astrophoto::auth::password;
use astrophoto::photos::pipeline;
use astrophoto::storage::S3Storage;
use astrophoto::users::queries as user_q;
use astrophoto::{Config, db};
use bytes::Bytes;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cfg = Config::from_env();

    let pool = db::connect(&cfg.database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let storage = Arc::new(
        S3Storage::new(
            cfg.s3_endpoint.as_deref(),
            &cfg.s3_region,
            &cfg.s3_bucket,
            &cfg.s3_access_key,
            &cfg.s3_secret_key,
            cfg.s3_path_style,
        )
        .await?,
    );

    let demo_email = "demo@astrophoto.example";
    let demo_user = match user_q::find_by_email(&pool, demo_email).await? {
        Some(u) => {
            tracing::info!(user_id = %u.id, "demo user already exists");
            u
        }
        None => {
            let hash = password::hash("demoaccount".into()).await?;
            let u = user_q::create_with_password(
                &pool,
                demo_email,
                "Demo Astrographer",
                &hash,
            )
            .await?;
            tracing::info!(user_id = %u.id, "demo user created");
            u
        }
    };

    let fixtures_dir = PathBuf::from("seeds/fixtures");
    let mut entries: Vec<_> = match std::fs::read_dir(&fixtures_dir) {
        Ok(it) => it.filter_map(Result::ok).collect(),
        Err(_) => {
            tracing::warn!(dir = %fixtures_dir.display(), "no fixtures dir; nothing to seed");
            return Ok(());
        }
    };
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) if n.ends_with(".jpg") || n.ends_with(".jpeg") || n.ends_with(".png") => {
                n.to_string()
            }
            _ => continue,
        };

        if photo_already_imported(&pool, demo_user.id, &name).await? {
            tracing::info!(file = %name, "already imported, skipping");
            continue;
        }

        let bytes = Bytes::from(
            std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?,
        );
        let mime = if name.ends_with(".png") {
            "image/png"
        } else {
            "image/jpeg"
        };
        let id = pipeline::process(
            &pool,
            storage.clone(),
            demo_user.id,
            &name,
            mime,
            None,
            None,
            bytes,
        )
        .await?;
        tracing::info!(file = %name, photo_id = %id, "imported");
    }

    Ok(())
}

async fn photo_already_imported(
    pool: &sqlx::PgPool,
    owner_id: uuid::Uuid,
    name: &str,
) -> Result<bool, astrophoto::AppError> {
    let row = sqlx::query!(
        "select 1 as one from photos where owner_id = $1 and original_name = $2 limit 1",
        owner_id,
        name
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}
```

- [ ] **Step 3: Create the fixtures directory anchor**

```bash
cd $ROOT/backend
mkdir -p seeds/fixtures
touch seeds/fixtures/.gitkeep
```

- [ ] **Step 4: Create the LICENSE placeholder**

`backend/seeds/fixtures/LICENSE.txt`:

```
Bundled fixtures in this directory are public-domain images from
NASA / ESA / JWST / Hubble / NOAO and similar US-government or
explicitly-released-into-the-public-domain sources.

Each file's source URL and credit line is recorded as a comment in
the file's name or in a sibling .txt file.

Drop your own JPEG / PNG astrophotos into this directory and re-run
`just seed` to import them into the demo user's gallery.
```

- [ ] **Step 5: Add `just seed` recipe**

Append to `justfile`:

```just
# Seed the dev database with demo content. Idempotent.
seed:
    cd backend && cargo run --bin seed
```

- [ ] **Step 6: Refresh sqlx cache and verify compile**

```bash
cd $ROOT/backend
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto cargo sqlx prepare -- --bins --lib --tests
cargo check --bin seed
```

Expected: clean.

- [ ] **Step 7: Commit**

```bash
cd $ROOT
git add backend/Cargo.toml backend/Cargo.lock backend/src/bin/seed.rs \
        backend/seeds/fixtures/.gitkeep backend/seeds/fixtures/LICENSE.txt \
        backend/.sqlx/ justfile
git commit -m "feat(backend): add 'just seed' demo importer"
```

---

## Task 6: Bundle a real PD-NASA fixture

**Files:**
- Create: `backend/seeds/fixtures/m42-orion.jpg` (fetched from a public-domain NASA source)
- Modify: `backend/seeds/fixtures/LICENSE.txt` (add specific credit line)

- [ ] **Step 1: Download a public-domain NASA astronomy image**

The "M42 Orion Nebula" or "M16 Eagle Nebula" Hubble images on Wikimedia Commons are PD-NASA. Pick one ~150-300 KB JPEG with embedded EXIF if available; if not, the JPEG headers from any NASA image will at least carry SOF dimensions.

Suggested source (verify license at fetch time, prefer the smaller "1024px" or "thumbnail" variant):

```bash
cd $ROOT/backend/seeds/fixtures
# Example — exact URL/filename TBD at implementation time; use any
# Wikimedia Commons NASA-credit JPEG ≤ 500 KB:
curl -L -o m42-orion.jpg \
  'https://upload.wikimedia.org/wikipedia/commons/thumb/2/2b/Orion_Nebula_-_Hubble_2006_mosaic_18000.jpg/800px-Orion_Nebula_-_Hubble_2006_mosaic_18000.jpg'
ls -la m42-orion.jpg
```

If the download fails or the URL has changed, pick any other Wikimedia
Commons "PD-NASA"-tagged astrophoto JPEG. Confirm the size is sane
(< 500 KB) so the repo stays light.

- [ ] **Step 2: Update LICENSE.txt**

Replace the contents of `backend/seeds/fixtures/LICENSE.txt`:

```
Bundled fixtures in this directory.

m42-orion.jpg
  Title:   Orion Nebula (M42) — Hubble 2006 mosaic (cropped 800px)
  Source:  https://commons.wikimedia.org/wiki/File:Orion_Nebula_-_Hubble_2006_mosaic_18000.jpg
  Credit:  NASA, ESA, M. Robberto (STScI/ESA) and the Hubble Space
           Telescope Orion Treasury Project Team
  License: Public domain (PD-NASA)

Drop your own JPEG / PNG astrophotos into this directory and re-run
`just seed` to import them into the demo user's gallery.
```

(If you sourced a different fixture in Step 1, swap the title /
source / credit lines accordingly.)

- [ ] **Step 3: Smoke run the seed**

Backend MUST be stopped (the seed connects to the same DB; concurrent
runs are fine but stopping main avoids confusion). MinIO must be up
(it is, from Phase 5).

```bash
cd $ROOT
APP_BIND=127.0.0.1:8080 \
APP_LOG=info \
APP_DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto \
APP_SESSION_DOMAIN=localhost APP_SESSION_SECURE=false \
APP_PUBLIC_BASE_URL=http://localhost:8080 \
APP_S3_ENDPOINT=http://localhost:9100 \
APP_S3_REGION=us-east-1 APP_S3_BUCKET=astrophoto \
APP_S3_ACCESS_KEY=minioadmin APP_S3_SECRET_KEY=minioadmin \
APP_S3_PATH_STYLE=true \
just seed
```

Expected: log lines `demo user created` then `imported file=m42-orion.jpg photo_id=<uuid>`.

Re-run the same command. Expected: `demo user already exists` and `already imported, skipping file=m42-orion.jpg`.

- [ ] **Step 4: Commit**

```bash
cd $ROOT
git add backend/seeds/fixtures/m42-orion.jpg backend/seeds/fixtures/LICENSE.txt
git commit -m "feat(seed): bundle PD-NASA M42 fixture"
```

---

## Task 7: Photo detail EXIF row builder (item b)

**Files:**
- Modify: `frontend/src/routes/photo/[slug]/+page.server.ts` (add the row builder, replace placeholder branch)

- [ ] **Step 1: Add the builder + helpers and use them in the load**

Replace the body of the UUID branch in `frontend/src/routes/photo/[slug]/+page.server.ts` with the following. Keep the canonical `ngc-7000-north-america-nebula` branch unchanged.

Add these helpers at module top (above `export const load`):

```ts
interface ExifRow {
  label: string;
  value: string;
  sublabel?: string;
  sublabelAccent?: boolean;
}

interface RealPhoto {
  id: string;
  owner_id: string;
  status: string;
  original_name: string;
  bytes: number;
  mime: string;
  width: number | null;
  height: number | null;
  taken_at: string | null;
  camera: string | null;
  lens: string | null;
  iso: number | null;
  exposure_s: number | null;
  focal_mm: number | null;
  target: string | null;
  caption: string | null;
  created_at: string;
}

function formatBytes(b: number): string {
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  return `${(b / 1024 / 1024).toFixed(1)} MB`;
}

function formatExposure(s: number): string {
  if (s >= 1) return `${s} s`;
  if (s <= 0) return `${s} s`;
  return `1/${Math.round(1 / s)} s`;
}

function formatDate(iso: string): string {
  const d = new Date(iso);
  if (isNaN(d.getTime())) return iso;
  return d.toISOString().split('T')[0]; // YYYY-MM-DD
}

function buildExifRows(p: RealPhoto): ExifRow[] {
  const rows: ExifRow[] = [];
  rows.push({ label: 'Original file', value: p.original_name });
  rows.push({ label: 'Size', value: formatBytes(p.bytes) });
  if (p.width != null && p.height != null) {
    rows.push({ label: 'Dimensions', value: `${p.width} × ${p.height}` });
  }
  if (p.target) rows.push({ label: 'Target', value: p.target });
  if (p.taken_at) rows.push({ label: 'Captured', value: formatDate(p.taken_at) });
  if (p.camera) rows.push({ label: 'Camera', value: p.camera });
  if (p.lens) rows.push({ label: 'Lens', value: p.lens });
  if (p.iso != null) rows.push({ label: 'ISO', value: String(p.iso) });
  if (p.exposure_s != null) {
    rows.push({ label: 'Exposure', value: formatExposure(p.exposure_s) });
  }
  if (p.focal_mm != null) {
    rows.push({ label: 'Focal', value: `${p.focal_mm} mm` });
  }
  return rows;
}
```

In the UUID branch of `load`, after fetching the photo JSON, build the
return shape using `buildExifRows`. The page's existing `data.photo`
shape includes an `exifRows` array; populate it from the helper:

```ts
return {
  photo: {
    slug: photo.id,
    target: photo.target ?? 'Untitled',
    targetSubtitle: '',
    captured: '',
    photographer: {
      name: 'User',
      initial: 'U',
      frames: 0,
      bortle: 0,
      location: '',
      caption: photo.caption ?? '',
      captionShort: photo.caption ?? ''
    },
    appreciations: 0,
    comments: 0,
    ratio: photo.width && photo.height ? photo.width / photo.height : 1.5,
    integration: '',
    thumbSrc1200: `${API}/api/photos/${photo.id}/thumb/1200`,
    exifRows: buildExifRows(photo)
  },
  isRich: false
};
```

In `+page.svelte`, find the `<ExifTable rows={exifRows} />` invocation
in the non-rich branch. Replace the existing fallback (which built rows
inline) with `data.photo.exifRows`.

- [ ] **Step 2: Verify check + lint**

```bash
cd $ROOT
pnpm -C frontend check
pnpm -C frontend lint
```

Expected: 0 errors / 0 warnings.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/routes/photo/[slug]/+page.server.ts frontend/src/routes/photo/[slug]/+page.svelte
git commit -m "feat(frontend): EXIF panel adapts to real photos with sparse fields"
```

---

## Task 8: Hero / grid dedup (item e) + drop "SHO" hardcode (item a)

**Files:**
- Modify: `frontend/src/routes/+page.server.ts`
- Modify: `frontend/src/routes/+page.svelte`

- [ ] **Step 1: Update `+page.server.ts` to split hero from rest**

Replace the real-photo branch:

```ts
if (realPhotos.length > 0) {
  const [hero, ...rest] = realPhotos;
  return {
    heroPhoto: {
      target: hero.target ?? 'Untitled',
      integration: '',
      photographer: ''
    },
    heroSrc: `${API}/api/photos/${hero.id}/thumb/1200`,
    photos: rest.map((p) => ({
      slug: p.id,
      target: p.target ?? 'Untitled',
      ratio: p.width && p.height ? p.width / p.height : 1.5,
      integration: '',
      photographer: '',
      photographerSlug: '',
      camera: '',
      thumbSrc: `${API}/api/photos/${p.id}/thumb/400`
    })),
    isReal: true
  };
}
```

- [ ] **Step 2: Update `+page.svelte` to use `heroSrc` and conditional tag**

In the hero photo block, set the `<Photo>` to use the new `heroSrc`:

```svelte
<Photo target={data.heroPhoto.target} src={data.heroSrc} />
```

(Use `src={data.heroSrc}` — when `isReal=false`, `data.heroSrc` is
undefined and `Photo` falls back to the gradient.)

For the "FRAME OF THE WEEK" overlay, replace the existing block with a
conditional:

```svelte
<div class="frame-of-the-week-tag">
  <div style="color: var(--accent)">FRAME OF THE WEEK</div>
  {#if data.isReal}
    <div style="color: var(--fg-primary)">{data.heroPhoto.target}</div>
  {:else}
    <div style="color: var(--fg-primary)">
      {data.heroPhoto.target} · {data.heroPhoto.integration}
    </div>
    <div style="color: var(--fg-muted)">Marie Dubois · Bortle 4</div>
  {/if}
</div>
```

Locate the masonry grid block — it iterates `data.photos`. No code
change needed there (when `data.photos` is empty, the loop renders
nothing). Adjust the surrounding wrapper if the design's section
heading "FROM THE COMMUNITY" is conditional on having content;
otherwise leave as-is (an empty grid block under the filter bar is
acceptable).

- [ ] **Step 3: Verify check + lint**

```bash
cd $ROOT
pnpm -C frontend check
pnpm -C frontend lint
```

Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/routes/+page.server.ts frontend/src/routes/+page.svelte
git commit -m "feat(frontend): dedup hero/grid + drop hardcoded SHO tag"
```

---

## Task 9: `/account/logout` server action

**Files:**
- Create: `frontend/src/routes/account/logout/+server.ts`

- [ ] **Step 1: Create the file**

```ts
import { redirect } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

const API = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080';

export const POST: RequestHandler = async ({ fetch, cookies, setHeaders }) => {
  const cookie = cookies
    .getAll()
    .map((c) => `${c.name}=${c.value}`)
    .join('; ');
  let res: Response;
  try {
    res = await fetch(`${API}/api/auth/logout`, {
      method: 'POST',
      headers: { Cookie: cookie }
    });
  } catch {
    // Backend unreachable — clear client state anyway by setting
    // an expired session cookie locally.
    setHeaders({
      'set-cookie':
        'session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0'
    });
    throw redirect(303, '/');
  }
  const setCookie = res.headers.get('set-cookie');
  if (setCookie) setHeaders({ 'set-cookie': setCookie });
  throw redirect(303, '/');
};
```

- [ ] **Step 2: Verify check**

```bash
cd $ROOT/frontend && pnpm check
```

Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
cd $ROOT
git add frontend/src/routes/account/logout/+server.ts
git commit -m "feat(frontend): add /account/logout POST handler"
```

---

## Task 10: AvatarMenu component

**Files:**
- Create: `frontend/src/lib/components/AvatarMenu.svelte`
- Modify: `frontend/src/lib/components/AppHeader.svelte` (use AvatarMenu when authenticated)

- [ ] **Step 1: Create AvatarMenu**

```svelte
<script lang="ts">
  interface Props {
    user: { id: string; displayName: string };
  }

  let { user }: Props = $props();

  let open = $state(false);
  let containerEl: HTMLDivElement | undefined = $state();

  function toggle() {
    open = !open;
  }

  function close() {
    open = false;
  }

  // Click outside + Escape close.
  $effect(() => {
    if (!open) return;
    const onDocClick = (e: MouseEvent) => {
      if (containerEl && !containerEl.contains(e.target as Node)) close();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close();
    };
    document.addEventListener('click', onDocClick);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('click', onDocClick);
      document.removeEventListener('keydown', onKey);
    };
  });

  let initial = $derived(user.displayName?.[0]?.toUpperCase() ?? 'U');
</script>

<div class="avatar-wrap" bind:this={containerEl}>
  <button
    type="button"
    class="avatar"
    aria-haspopup="menu"
    aria-expanded={open}
    onclick={toggle}
  >
    {initial}
  </button>

  {#if open}
    <div class="menu" role="menu">
      <div class="menu-greeting">
        <span class="t-meta" style="color: var(--fg-muted);">Signed in as</span>
        <div style="color: var(--fg-primary); font-size: 13px;">{user.displayName}</div>
      </div>
      <div class="menu-divider"></div>
      <a href="/u/{user.id}" class="menu-item" role="menuitem" onclick={close}>Profile</a>
      <a href="/upload" class="menu-item" role="menuitem" onclick={close}>Upload</a>
      <div class="menu-divider"></div>
      <form method="POST" action="/account/logout">
        <button type="submit" class="menu-item menu-item-button" role="menuitem">
          Sign out
        </button>
      </form>
    </div>
  {/if}
</div>

<style>
  .avatar-wrap {
    position: relative;
  }

  .avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--accent);
    color: var(--accent-ink);
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-display);
    font-size: 15px;
    border: 0;
    cursor: pointer;
    padding: 0;
  }

  .menu {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    width: 220px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-default);
    border-radius: var(--r-md);
    box-shadow: var(--shadow-md);
    z-index: 100;
    padding: 8px 0;
    animation: menu-in 150ms var(--ease-out);
  }

  @keyframes menu-in {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .menu-greeting {
    padding: 12px 16px;
    line-height: 1.4;
  }

  .menu-divider {
    height: 1px;
    background: var(--border-subtle);
    margin: 4px 0;
  }

  .menu-item,
  .menu-item-button {
    display: block;
    width: 100%;
    padding: 10px 16px;
    text-align: left;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--fg-secondary);
    background: transparent;
    border: 0;
    cursor: pointer;
    text-decoration: none;
  }

  .menu-item:hover,
  .menu-item-button:hover {
    background: var(--bg-raised);
    color: var(--fg-primary);
  }

  form {
    margin: 0;
  }
</style>
```

- [ ] **Step 2: Use AvatarMenu in AppHeader**

In `frontend/src/lib/components/AppHeader.svelte`, find the `{#if user}`
block where the current avatar `<a>` is rendered. Replace the avatar
link with `<AvatarMenu {user} />`.

Add the import at the top:

```svelte
import AvatarMenu from './AvatarMenu.svelte';
```

The auth branch becomes:

```svelte
{#if user}
  <a href="/upload" class="btn btn-secondary btn-sm">Upload</a>
  <AvatarMenu {user} />
{:else}
  <a href="/signin" class="nav-link">Sign in</a>
  <a href="/signup" class="btn btn-primary btn-sm">Create account</a>
{/if}
```

- [ ] **Step 3: Verify check + lint**

```bash
cd $ROOT
pnpm -C frontend check
pnpm -C frontend lint
pnpm -C frontend build
```

Expected: 0 errors, build succeeds.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/lib/components/AvatarMenu.svelte frontend/src/lib/components/AppHeader.svelte
git commit -m "feat(frontend): add avatar dropdown with Sign out"
```

---

## Task 11: Wire `/u/[username]` to the new users endpoint

**Files:**
- Modify: `frontend/src/routes/u/[username]/+page.server.ts`

- [ ] **Step 1: Update the UUID branch to fetch UserPublic + photos**

Replace the existing UUID branch with one that calls both endpoints:

```ts
const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const API = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080';

if (UUID_RE.test(params.username)) {
  let displayName = 'User';
  let photoCount = 0;
  let memberSince = '';
  try {
    const res = await fetch(`${API}/api/users/${params.username}`);
    if (res.ok) {
      const u = (await res.json()) as {
        id: string;
        display_name: string;
        created_at: string;
        photo_count: number;
      };
      displayName = u.display_name;
      photoCount = u.photo_count;
      memberSince = new Date(u.created_at).getFullYear().toString();
    } else if (res.status === 404) {
      throw error(404, 'User not found');
    }
  } catch (e) {
    // Network error: fall through with defaults rather than 500.
    if (e && (e as { status?: number }).status) throw e;
  }

  let photos: Array<{ slug: string; target: string; thumbSrc: string }> = [];
  try {
    const res = await fetch(`${API}/api/photos?owner_id=${params.username}&limit=24`);
    if (res.ok) {
      const body = (await res.json()) as { photos: Array<{ id: string; target: string | null }> };
      photos = body.photos.map((p) => ({
        slug: p.id,
        target: p.target ?? 'Untitled',
        thumbSrc: `${API}/api/photos/${p.id}/thumb/400`
      }));
    }
  } catch {
    // ignore
  }

  // Pick out the first name + italicized surname purely visually.
  const parts = displayName.split(' ');
  const firstName = parts[0] ?? displayName;
  const surnameItalic = parts.slice(1).join(' ');

  return {
    user: {
      username: params.username,
      displayName,
      firstName,
      surnameItalic,
      initial: displayName[0]?.toUpperCase() ?? 'U',
      about: '',
      frames: photoCount,
      integrationTotal: '',
      followers: 0,
      collections: 0,
      lat: '',
      long: '',
      bortle: 0,
      sqm: 0,
      equipment: { scope: '', camera: '', mount: '', filters: '' },
      memberSince
    },
    photos,
    isReal: true
  };
}
```

(Keep the `marie-dubois` branch unchanged. Add a final `else { throw error(404, 'User not found'); }` for unknown slugs.)

The existing `+page.svelte` should consume the new `photos` array. If it iterates `MARIE.frames` or similar placeholder field for the grid, switch to `data.photos`. If a code change is needed in `+page.svelte`, inline it now: pass `src={p.thumbSrc}` to the gallery `<Photo>` instances.

- [ ] **Step 2: Smoke check**

```bash
cd $ROOT
pnpm -C frontend check
pnpm -C frontend build
```

Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/routes/u/[username]/+page.server.ts frontend/src/routes/u/[username]/+page.svelte
git commit -m "feat(frontend): /u/[uuid] resolves real display_name via /api/users/:id"
```

---

## Task 12: Final cross-cutting verification + browser smoke

**Files:**
- None (verification only)

- [ ] **Step 1: Restart backend and frontend**

Kill any running instances first:

```bash
pkill -f "target/debug/astrophoto" 2>/dev/null
pkill -f "vite dev" 2>/dev/null
```

Start backend (terminal A):

```bash
cd $ROOT/backend
APP_BIND=127.0.0.1:8080 \
APP_LOG=info \
APP_DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto \
APP_SESSION_DOMAIN=localhost APP_SESSION_SECURE=false \
APP_PUBLIC_BASE_URL=http://localhost:8080 \
APP_S3_ENDPOINT=http://localhost:9100 \
APP_S3_REGION=us-east-1 APP_S3_BUCKET=astrophoto \
APP_S3_ACCESS_KEY=minioadmin APP_S3_SECRET_KEY=minioadmin \
APP_S3_PATH_STYLE=true \
cargo run --bin astrophoto
```

Start frontend (terminal B):

```bash
cd $ROOT/frontend
VITE_API_BASE_URL=http://localhost:8080 pnpm dev
```

Wait for both to be reachable on :8080 and :5173.

- [ ] **Step 2: Run `just seed`**

Terminal C:

```bash
cd $ROOT
APP_DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto \
APP_S3_ENDPOINT=http://localhost:9100 \
APP_S3_REGION=us-east-1 APP_S3_BUCKET=astrophoto \
APP_S3_ACCESS_KEY=minioadmin APP_S3_SECRET_KEY=minioadmin \
APP_S3_PATH_STYLE=true \
APP_BIND=unused APP_LOG=info APP_SESSION_DOMAIN=localhost APP_SESSION_SECURE=false APP_PUBLIC_BASE_URL=http://localhost:8080 \
just seed
```

Expected logs: `demo user created` then `imported file=m42-orion.jpg`.

- [ ] **Step 3: Verify endpoints via curl**

```bash
# Demo user lookup — find the demo user's id from the photos list:
DEMO_USER_ID=$(curl -s 'http://localhost:8080/api/photos?limit=1' \
  | grep -oE '"owner_id":"[^"]+"' | head -1 | cut -d'"' -f4)
echo "demo user id: $DEMO_USER_ID"

# Hit /api/users/<id>
curl -s "http://localhost:8080/api/users/$DEMO_USER_ID"
```

Expected: a JSON with `display_name: "Demo Astrographer"`, `photo_count: 1`, no `email`.

- [ ] **Step 4: Browser smoke checklist**

Open `http://localhost:5173/` and verify in order:

1. Gallery hero shows the M42 demo photo, with tag `FRAME OF THE WEEK / Orion Nebula` (or the target you chose). NO `· SHO` suffix.
2. Grid below is empty (because there's only 1 real photo and it's the hero).
3. Click hero → `/photo/<uuid>` opens with the M42 image and an EXIF panel showing `Original file / Size / Dimensions` plus any extra fields the JPEG carries (Camera, Lens if present from Hubble metadata).
4. Sign up a fresh user via `/signup`. Confirm redirect to `/`. Header shows the avatar circle.
5. Click the avatar. Dropdown opens with greeting, `Profile`, `Upload`, divider, `Sign out`. Click outside → closes.
6. Click `Profile`. Lands at `/u/<your-uuid>`. Title shows your display name.
7. Click avatar → `Sign out`. Redirected to `/`. Header shows `Sign in / Create account`.

If any step fails, stop and fix before merging.

- [ ] **Step 5: Final `just check`**

```bash
cd $ROOT && just check
cd backend && cargo test
```

Expected: all green.

- [ ] **Step 6: Merge**

```bash
cd $ROOT
git checkout main
git merge --no-ff feat/phase-6-polish -m "Merge feat/phase-6-polish: UX polish + seed + users endpoint"
git tag -a v0.5.0-polish -m "Astrophoto v0.5.0 — polish, seed, /api/users/:id"
git branch -d feat/phase-6-polish
```

---

## Self-Review

**Spec coverage:**
- Item a (drop SHO): Task 8 ✓
- Item b (graceful EXIF): Task 7 ✓
- Item c (`/api/users/:id`): Tasks 1, 2, 3 ✓
- Item d (avatar dropdown + Sign out): Tasks 9, 10 ✓
- Item e (hero/grid dedup): Task 8 ✓
- Item f (`just seed` + bundled fixture): Tasks 4, 5, 6 ✓
- `/u/[uuid]` wiring to new endpoint: Task 11 ✓
- Smoke verification: Task 12 ✓

**Placeholder scan:** none — every step contains exact code or commands. No "TBD" except the URL inside Task 6 Step 1, which is explicitly flagged as "verify license at fetch time" and gives a concrete example URL plus a fallback (any Wikimedia PD-NASA tagged JPEG).

**Type consistency:**
- `UserPublic { id: String, display_name: String, created_at: String, photo_count: i64 }` — defined in Task 2, consumed by Task 11.
- `RealPhoto { ... target: string | null, ... }` interface in Task 7 matches the `PhotoSummary` shape from Phase 5's API client.
- `pipeline::process` signature: `(pool, storage, owner_id, original_name, mime, target, caption, bytes) -> Result<Uuid, AppError>` — same in Task 4 (definition) and Task 5 (seed call site).
- AvatarMenu's `user: { id: string; displayName: string }` matches `event.locals.user` from Phase 4's `app.d.ts`.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-05-02-phase-6-polish.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — I execute tasks in this session using executing-plans, batch execution with checkpoints for review.

**Which approach?**
