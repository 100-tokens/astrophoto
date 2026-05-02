# Phase 5 — Photo Upload Pipeline

**Goal:** Authenticated users can upload an astrophoto via `/upload`, the
backend extracts EXIF + generates thumbnails, and the photo appears in
the gallery, the user's profile, and a detail page with the real image.

**Architecture:** A `Storage` trait abstracts S3-compatible blob storage
(R2 in prod, MinIO in dev, in-memory in tests). The upload handler
streams the original to storage, inserts a `photos` row with
`status = 'processing'`, then spawns a `tokio::task::spawn_blocking` that
parses EXIF (`kamadak-exif`), generates 400 px and 1200 px JPEG thumbs
(`image` crate), persists thumbnails to storage, and updates the row to
`status = 'ready'`. Photo bytes flow through the backend
(`/api/photos/<id>/thumb/<size>`) — no presigned URLs in MVP.

**Tech stack:** axum 0.7, sqlx 0.8, aws-sdk-s3 v1, kamadak-exif 0.5,
image 0.25, tokio spawn_blocking, multer (multipart parser used by axum).

**Spec reference:** `docs/superpowers/specs/2026-05-01-astrophoto-bootstrap-design.md` §Image pipeline.

---

## Working assumptions

- Branch: `feat/phase-5-photos`. Multiple commits per batch. Merge to
  `main` only at the end.
- Postgres running on `localhost:5434`, MinIO on `localhost:9100`
  (`minioadmin`/`minioadmin`), bucket `astrophoto` will be created on
  backend boot if missing.
- `clippy::unwrap_used` and `clippy::expect_used` lints denied in lib code.
- After every batch: `just check` and `cargo test` MUST pass.
- `cargo sqlx prepare` after any new SQL macro; commit `.sqlx/`.

---

## Batch 1 — Storage trait + MinIO bucket auto-creation

### Task 1.1 — `backend/src/storage/{mod,s3,memory}.rs`

**Files:**
- Create `backend/src/storage/mod.rs`
- Create `backend/src/storage/s3.rs`
- Create `backend/src/storage/memory.rs`
- Modify `backend/src/lib.rs` (`pub mod storage`)

**`backend/src/storage/mod.rs`:**

```rust
//! Object storage abstraction. S3-compatible (R2 in prod, MinIO in dev).
//! In-memory impl for tests.

use async_trait::async_trait;
use bytes::Bytes;

use crate::AppError;

pub mod memory;
pub mod s3;

pub use memory::MemoryStorage;
pub use s3::S3Storage;

#[async_trait]
pub trait Storage: Send + Sync + 'static {
    /// Store `body` at `key` with the given content type. Overwrites if exists.
    async fn put(&self, key: &str, content_type: &str, body: Bytes) -> Result<(), AppError>;

    /// Retrieve an object's bytes. None if missing.
    async fn get(&self, key: &str) -> Result<Option<Bytes>, AppError>;

    /// Delete an object. Idempotent (no error if missing).
    async fn delete(&self, key: &str) -> Result<(), AppError>;
}
```

Add `async-trait = "0.1"` and `bytes = "1"` to `[dependencies]` in
`Cargo.toml` if not present.

**`backend/src/storage/memory.rs`:**

```rust
use async_trait::async_trait;
use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Mutex;

use super::Storage;
use crate::AppError;

#[derive(Default)]
pub struct MemoryStorage {
    inner: Mutex<HashMap<String, (String, Bytes)>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Storage for MemoryStorage {
    async fn put(&self, key: &str, content_type: &str, body: Bytes) -> Result<(), AppError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|e| AppError::Internal(format!("memory lock: {e}")))?;
        g.insert(key.to_string(), (content_type.to_string(), body));
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<Bytes>, AppError> {
        let g = self
            .inner
            .lock()
            .map_err(|e| AppError::Internal(format!("memory lock: {e}")))?;
        Ok(g.get(key).map(|(_, b)| b.clone()))
    }

    async fn delete(&self, key: &str) -> Result<(), AppError> {
        let mut g = self
            .inner
            .lock()
            .map_err(|e| AppError::Internal(format!("memory lock: {e}")))?;
        g.remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn put_get_delete_round_trip() {
        let s = MemoryStorage::new();
        let body = Bytes::from_static(b"hello");
        s.put("k", "text/plain", body.clone()).await.unwrap();
        assert_eq!(s.get("k").await.unwrap().unwrap(), body);
        s.delete("k").await.unwrap();
        assert!(s.get("k").await.unwrap().is_none());
    }
}
```

**`backend/src/storage/s3.rs`:**

```rust
use async_trait::async_trait;
use aws_sdk_s3::{Client, config::{Builder, Credentials, Region}, primitives::ByteStream, types::CreateBucketConfiguration};
use bytes::Bytes;

use super::Storage;
use crate::AppError;

pub struct S3Storage {
    client: Client,
    bucket: String,
}

impl S3Storage {
    pub async fn new(
        endpoint: Option<&str>,
        region: &str,
        bucket: &str,
        access_key: &str,
        secret_key: &str,
        path_style: bool,
    ) -> Result<Self, AppError> {
        let creds = Credentials::new(access_key, secret_key, None, None, "static");
        let region_owned = Region::new(region.to_string());
        let mut builder = Builder::new()
            .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
            .credentials_provider(creds)
            .region(region_owned)
            .force_path_style(path_style);
        if let Some(ep) = endpoint {
            builder = builder.endpoint_url(ep);
        }
        let client = Client::from_conf(builder.build());
        let s = S3Storage {
            client,
            bucket: bucket.to_string(),
        };
        s.ensure_bucket().await?;
        Ok(s)
    }

    /// Create the bucket if it doesn't already exist. Idempotent.
    async fn ensure_bucket(&self) -> Result<(), AppError> {
        let head = self.client.head_bucket().bucket(&self.bucket).send().await;
        if head.is_ok() {
            return Ok(());
        }
        // Try to create. MinIO does not require LocationConstraint;
        // most S3 regions do, except us-east-1 where it must be omitted.
        let mut req = self.client.create_bucket().bucket(&self.bucket);
        let region = self
            .client
            .config()
            .region()
            .map(|r| r.as_ref())
            .unwrap_or("");
        if !region.is_empty() && region != "us-east-1" {
            let cfg = CreateBucketConfiguration::builder()
                .location_constraint(
                    aws_sdk_s3::types::BucketLocationConstraint::from(region),
                )
                .build();
            req = req.create_bucket_configuration(cfg);
        }
        req.send()
            .await
            .map(|_| ())
            .map_err(|e| AppError::Internal(format!("create bucket: {e}")))
    }
}

#[async_trait]
impl Storage for S3Storage {
    async fn put(&self, key: &str, content_type: &str, body: Bytes) -> Result<(), AppError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .body(ByteStream::from(body))
            .send()
            .await
            .map(|_| ())
            .map_err(|e| AppError::Internal(format!("s3 put: {e}")))
    }

    async fn get(&self, key: &str) -> Result<Option<Bytes>, AppError> {
        match self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
        {
            Ok(out) => {
                let bytes = out
                    .body
                    .collect()
                    .await
                    .map_err(|e| AppError::Internal(format!("s3 read body: {e}")))?
                    .into_bytes();
                Ok(Some(bytes))
            }
            Err(e) => {
                let svc_err = e.into_service_error();
                if svc_err.is_no_such_key() {
                    Ok(None)
                } else {
                    Err(AppError::Internal(format!("s3 get: {svc_err}")))
                }
            }
        }
    }

    async fn delete(&self, key: &str) -> Result<(), AppError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map(|_| ())
            .map_err(|e| AppError::Internal(format!("s3 delete: {e}")))
    }
}
```

> Notes:
> - The `aws_sdk_s3` v1 API may have changed. If `force_path_style` doesn't
>   exist on the builder, look for `force_path_style_addressing(true)` or
>   similar. The plan's signatures are best-effort against v1.x.
> - `is_no_such_key()` is on `GetObjectError` in v1; if the call shape
>   changed, use a string match on the error code instead.

### Task 1.2 — `AppState` carries `Arc<dyn Storage>`

Modify `backend/src/http/mod.rs`:

```rust
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    pub storage: Arc<dyn crate::storage::Storage>,
}

pub fn router(pool: PgPool, config: Config, storage: Arc<dyn crate::storage::Storage>) -> Router {
    let state = AppState { pool, config: Arc::new(config), storage };
    // ... existing routes
}
```

Modify `backend/src/main.rs`:

```rust
use std::sync::Arc;
use astrophoto::storage::S3Storage;
// ...
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
let app = http::router(pool, cfg.clone(), storage)
    .layer(http::cors_layer("http://localhost:5173"))
    .layer(TraceLayer::new_for_http());
```

Update `backend/tests/healthz.rs` and `backend/tests/auth.rs` to pass a
`MemoryStorage` (or any in-memory storage trait obj) to `http::router`.

### Quality gate

```bash
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto cargo check --manifest-path backend/Cargo.toml
cargo test --manifest-path backend/Cargo.toml --lib storage
just check
```

### Commits

1. `feat(backend/storage): add Storage trait + S3 + Memory impls`
2. `feat(backend): wire Storage into AppState + boot bucket auto-creation`

---

## Batch 2 — EXIF + thumbnail pipeline

### Task 2.1 — `backend/src/photos/exif.rs`

```rust
//! EXIF parsing. Runs in spawn_blocking; `kamadak-exif` is sync.

use std::io::Cursor;

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::Serialize;
use serde_json::Value as Json;

use crate::AppError;

#[derive(Default, Debug, Serialize)]
pub struct ExifData {
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub iso: Option<i32>,
    pub exposure_s: Option<f64>,
    pub focal_mm: Option<f64>,
    pub taken_at: Option<DateTime<Utc>>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    /// Raw payload as JSON for full preservation
    pub raw: Json,
}

/// Parse EXIF synchronously. Caller wraps in `spawn_blocking`.
pub fn parse_blocking(bytes: &[u8]) -> Result<ExifData, AppError> {
    let exif_reader = exif::Reader::new()
        .read_from_container(&mut Cursor::new(bytes))
        .ok();
    let mut data = ExifData::default();
    let mut raw = serde_json::Map::new();

    if let Some(ref reader) = exif_reader {
        for f in reader.fields() {
            let key = format!("{}", f.tag);
            let val = f.display_value().with_unit(reader).to_string();
            raw.insert(key, Json::String(val));
        }
        data.camera = read_string(reader, exif::Tag::Model);
        data.lens = read_string(reader, exif::Tag::LensModel);
        data.iso = read_int(reader, exif::Tag::PhotographicSensitivity);
        data.exposure_s = read_rational(reader, exif::Tag::ExposureTime);
        data.focal_mm = read_rational(reader, exif::Tag::FocalLength);
        data.taken_at = read_datetime(reader, exif::Tag::DateTimeOriginal);
        data.width = read_int(reader, exif::Tag::PixelXDimension);
        data.height = read_int(reader, exif::Tag::PixelYDimension);
    }
    data.raw = Json::Object(raw);
    Ok(data)
}

fn read_string(r: &exif::Exif, tag: exif::Tag) -> Option<String> {
    let f = r.get_field(tag, exif::In::PRIMARY)?;
    Some(f.display_value().to_string().trim_matches('"').to_string())
}

fn read_int(r: &exif::Exif, tag: exif::Tag) -> Option<i32> {
    let f = r.get_field(tag, exif::In::PRIMARY)?;
    f.value.get_uint(0).map(|n| n as i32)
}

fn read_rational(r: &exif::Exif, tag: exif::Tag) -> Option<f64> {
    let f = r.get_field(tag, exif::In::PRIMARY)?;
    if let exif::Value::Rational(ref v) = f.value {
        v.first().map(|r| r.to_f64())
    } else {
        None
    }
}

fn read_datetime(r: &exif::Exif, tag: exif::Tag) -> Option<DateTime<Utc>> {
    let f = r.get_field(tag, exif::In::PRIMARY)?;
    if let exif::Value::Ascii(ref bytes) = f.value {
        let s = bytes
            .first()
            .and_then(|v| std::str::from_utf8(v).ok())?;
        let dt = NaiveDateTime::parse_from_str(s, "%Y:%m:%d %H:%M:%S").ok()?;
        Some(Utc.from_utc_datetime(&dt))
    } else {
        None
    }
}
```

> The `kamadak-exif` API may differ slightly. Adapt as needed; the goal
> is to read `Camera`, `Lens`, `ISO`, `ExposureTime`, `FocalLength`,
> `DateTimeOriginal`, `PixelXDimension`, `PixelYDimension`. If a field
> is missing or unparseable, leave it `None` and continue.

### Task 2.2 — `backend/src/photos/thumbs.rs`

```rust
//! JPEG thumbnail generation. Runs in spawn_blocking; `image` is sync.

use std::io::Cursor;

use bytes::Bytes;
use image::{ImageFormat, imageops::FilterType};

use crate::AppError;

#[derive(Debug, Clone, Copy)]
pub struct Thumb {
    pub size: u32,
    pub bytes: Bytes,
    pub width: u32,
    pub height: u32,
}

/// Decode an image, resize to `max_size` (longest side), encode JPEG q=88.
/// Returns the encoded bytes plus actual width/height.
pub fn generate_blocking(input: &[u8], max_size: u32) -> Result<Thumb, AppError> {
    let img =
        image::load_from_memory(input).map_err(|e| AppError::Validation(format!("decode: {e}")))?;
    let resized = if img.width().max(img.height()) <= max_size {
        img
    } else {
        img.resize(max_size, max_size, FilterType::Lanczos3)
    };
    let (w, h) = (resized.width(), resized.height());
    let mut out = Cursor::new(Vec::with_capacity(64 * 1024));
    resized
        .to_rgb8()
        .write_to(&mut out, ImageFormat::Jpeg)
        .map_err(|e| AppError::Internal(format!("encode: {e}")))?;
    Ok(Thumb {
        size: max_size,
        bytes: Bytes::from(out.into_inner()),
        width: w,
        height: h,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a synthetic 800×600 RGB image, encode as JPEG, resize.
    fn make_test_jpeg() -> Vec<u8> {
        use image::{DynamicImage, RgbImage};
        let img = DynamicImage::ImageRgb8(RgbImage::from_fn(800, 600, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        }));
        let mut buf = Cursor::new(Vec::new());
        img.write_to(&mut buf, ImageFormat::Jpeg).unwrap();
        buf.into_inner()
    }

    #[test]
    fn resize_to_400() {
        let jpeg = make_test_jpeg();
        let t = generate_blocking(&jpeg, 400).unwrap();
        assert_eq!(t.size, 400);
        assert!(t.width <= 400 && t.height <= 400);
        assert!(!t.bytes.is_empty());
    }

    #[test]
    fn smaller_input_unchanged_dims() {
        let small_jpeg = {
            use image::{DynamicImage, RgbImage};
            let img =
                DynamicImage::ImageRgb8(RgbImage::from_fn(200, 150, |_, _| image::Rgb([0, 0, 0])));
            let mut buf = Cursor::new(Vec::new());
            img.write_to(&mut buf, ImageFormat::Jpeg).unwrap();
            buf.into_inner()
        };
        let t = generate_blocking(&small_jpeg, 400).unwrap();
        assert_eq!(t.width, 200);
        assert_eq!(t.height, 150);
    }
}
```

### Task 2.3 — Wire `photos::exif` and `photos::thumbs` modules

`backend/src/photos/mod.rs`:

```rust
pub mod exif;
pub mod thumbs;
```

`backend/src/lib.rs`:

```rust
pub mod photos;
```

### Quality gate

```bash
cd backend && cargo test --lib photos -- --nocapture
```

Both thumb tests pass. (No EXIF unit test — we exercise EXIF in the
integration test in Batch 3 against a real fixture.)

### Commits

1. `feat(backend/photos): add EXIF parser`
2. `feat(backend/photos): add JPEG thumbnail generator`

---

## Batch 3 — Upload + photo queries + serve endpoints

### Task 3.1 — `backend/src/photos/queries.rs`

Common queries for photos:

```rust
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::AppError;

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
}

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
                            target, caption, status)
        values ($1, $2, $3, $4, $5, $6, $7, 'processing')
        returning id
        "#,
        owner_id, storage_key, original_name, bytes, mime, target, caption,
    )
    .fetch_one(pool)
    .await?;
    Ok(row.id)
}

pub async fn mark_ready(
    pool: &PgPool,
    id: Uuid,
    width: i32,
    height: i32,
    exif: &crate::photos::exif::ExifData,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"
        update photos set
            status='ready',
            width=$2, height=$3,
            taken_at=$4, camera=$5, lens=$6, iso=$7,
            exposure_s=$8, focal_mm=$9,
            exif_json=$10
        where id=$1
        "#,
        id,
        width, height,
        exif.taken_at, exif.camera, exif.lens, exif.iso,
        exif.exposure_s, exif.focal_mm,
        exif.raw,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_failed(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    sqlx::query!("update photos set status='failed' where id=$1", id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn insert_thumbnail(
    pool: &PgPool,
    photo_id: Uuid,
    size: i32,
    storage_key: &str,
    bytes: i64,
) -> Result<(), AppError> {
    sqlx::query!(
        r#"insert into thumbnails (photo_id, size, storage_key, bytes) values ($1,$2,$3,$4)
           on conflict (photo_id, size) do update set storage_key=$3, bytes=$4"#,
        photo_id, size, storage_key, bytes,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<PhotoRow>, AppError> {
    let row = sqlx::query_as!(
        PhotoRow,
        r#"
        select id, owner_id, storage_key, original_name, bytes, mime,
               width, height, taken_at, camera, lens, iso, exposure_s, focal_mm,
               target, caption, status, created_at
        from photos where id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn list_by_owner(pool: &PgPool, owner_id: Uuid, limit: i64) -> Result<Vec<PhotoRow>, AppError> {
    let rows = sqlx::query_as!(
        PhotoRow,
        r#"
        select id, owner_id, storage_key, original_name, bytes, mime,
               width, height, taken_at, camera, lens, iso, exposure_s, focal_mm,
               target, caption, status, created_at
        from photos
        where owner_id = $1 and status = 'ready'
        order by created_at desc
        limit $2
        "#,
        owner_id, limit
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_recent_public(pool: &PgPool, limit: i64) -> Result<Vec<PhotoRow>, AppError> {
    let rows = sqlx::query_as!(
        PhotoRow,
        r#"
        select id, owner_id, storage_key, original_name, bytes, mime,
               width, height, taken_at, camera, lens, iso, exposure_s, focal_mm,
               target, caption, status, created_at
        from photos
        where status = 'ready'
        order by created_at desc
        limit $1
        "#,
        limit
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn thumb_storage_key(
    pool: &PgPool,
    photo_id: Uuid,
    size: i32,
) -> Result<Option<String>, AppError> {
    let row = sqlx::query!(
        "select storage_key from thumbnails where photo_id=$1 and size=$2",
        photo_id, size
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.storage_key))
}
```

### Task 3.2 — `POST /api/photos` upload handler

`backend/src/photos/upload.rs`:

```rust
use std::sync::Arc;

use axum::{
    Json,
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
};
use bytes::Bytes;
use serde::Serialize;
use uuid::Uuid;

use crate::auth::middleware::CurrentUser;
use crate::http::AppState;
use crate::photos::{exif, queries, thumbs};
use crate::storage::Storage;
use crate::AppError;

const MAX_BYTES: usize = 50 * 1024 * 1024; // 50 MB
const ALLOWED_MIMES: &[&str] = &["image/jpeg", "image/png", "image/tiff"];
const THUMB_SIZES: &[u32] = &[400, 1200];

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
            _ => {} // ignore unknown fields
        }
    }

    let bytes = file_bytes.ok_or_else(|| AppError::Validation("missing file".into()))?;
    if !ALLOWED_MIMES.contains(&mime.as_str()) {
        return Err(AppError::Validation(format!("unsupported mime: {mime}")));
    }

    let photo_id = Uuid::new_v4();
    let storage_key = format!("originals/{photo_id}");
    state
        .storage
        .put(&storage_key, &mime, bytes.clone())
        .await?;

    let id = queries::insert_processing(
        &state.pool,
        user.id,
        &storage_key,
        &filename,
        bytes.len() as i64,
        &mime,
        target.as_deref(),
        caption.as_deref(),
    )
    .await?;

    // Spawn background processing.
    let pool = state.pool.clone();
    let storage = state.storage.clone();
    let bytes_for_proc = bytes.clone();
    tokio::spawn(async move {
        if let Err(e) = process_photo(&pool, storage, id, bytes_for_proc).await {
            tracing::error!(photo_id=%id, error=%e, "photo processing failed");
            let _ = queries::mark_failed(&pool, id).await;
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(UploadResponse {
            id: id.to_string(),
            status: "processing".into(),
        }),
    ))
}

async fn process_photo(
    pool: &sqlx::PgPool,
    storage: Arc<dyn Storage>,
    id: Uuid,
    bytes: Bytes,
) -> Result<(), AppError> {
    let bytes_for_blocking = bytes.clone();
    let parsed = tokio::task::spawn_blocking(move || {
        let exif_data = exif::parse_blocking(&bytes_for_blocking)?;
        let mut thumbs = Vec::with_capacity(THUMB_SIZES.len());
        for size in THUMB_SIZES {
            thumbs.push(thumbs::generate_blocking(&bytes_for_blocking, *size)?);
        }
        Ok::<_, AppError>((exif_data, thumbs))
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking join: {e}")))??;

    let (exif_data, thumbs_out) = parsed;

    // Pick the largest as the canonical width/height (input image size,
    // since smaller-than-max thumbnails preserve original dimensions).
    let (full_w, full_h) = thumbs_out
        .iter()
        .max_by_key(|t| t.size)
        .map(|t| (t.width as i32, t.height as i32))
        .unwrap_or((0, 0));

    for thumb in thumbs_out {
        let key = format!("thumbs/{id}/{}", thumb.size);
        let len = thumb.bytes.len() as i64;
        storage.put(&key, "image/jpeg", thumb.bytes).await?;
        queries::insert_thumbnail(pool, id, thumb.size as i32, &key, len).await?;
    }

    queries::mark_ready(pool, id, full_w, full_h, &exif_data).await?;
    Ok(())
}
```

> Note: the `bytes_for_blocking` clone is moved into `spawn_blocking`. Both
> the EXIF parse and all thumbnail generations run inside one
> `spawn_blocking` call to amortize the thread-pool dispatch.

### Task 3.3 — Photo detail + thumbnail serving + listing endpoints

`backend/src/photos/get.rs`:

```rust
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use serde::Serialize;
use uuid::Uuid;

use crate::AppError;
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
        }
    }
}

pub async fn handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<PhotoDetail>, AppError> {
    let row = queries::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Json(row.into()))
}
```

`backend/src/photos/serve.rs` — stream thumbnail bytes:

```rust
use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::AppError;
use crate::http::AppState;
use crate::photos::queries;

pub async fn thumb(
    State(state): State<AppState>,
    Path((id, size)): Path<(Uuid, i32)>,
) -> Result<Response, AppError> {
    if !matches!(size, 400 | 1200) {
        return Err(AppError::Validation("size must be 400 or 1200".into()));
    }
    let key = queries::thumb_storage_key(&state.pool, id, size)
        .await?
        .ok_or(AppError::NotFound)?;
    let bytes = state.storage.get(&key).await?.ok_or(AppError::NotFound)?;
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "image/jpeg".parse().unwrap());
    headers.insert(header::CACHE_CONTROL, "public, max-age=31536000, immutable".parse().unwrap());
    Ok((headers, Body::from(bytes)).into_response())
}
```

`backend/src/photos/list.rs`:

```rust
use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppError;
use crate::http::AppState;
use crate::photos::queries;
use crate::photos::get::PhotoDetail;

#[derive(Deserialize)]
pub struct ListQuery {
    pub owner_id: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Serialize)]
pub struct ListResponse {
    pub photos: Vec<PhotoDetail>,
}

pub async fn handler(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListResponse>, AppError> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);
    let rows = match q.owner_id {
        Some(id) => queries::list_by_owner(&state.pool, id, limit).await?,
        None => queries::list_recent_public(&state.pool, limit).await?,
    };
    Ok(Json(ListResponse {
        photos: rows.into_iter().map(Into::into).collect(),
    }))
}
```

`backend/src/photos/mod.rs`:

```rust
pub mod exif;
pub mod get;
pub mod list;
pub mod queries;
pub mod serve;
pub mod thumbs;
pub mod upload;
```

Mount routes in `backend/src/http/mod.rs`:

```rust
.route("/api/photos", post(crate::photos::upload::handler).get(crate::photos::list::handler))
.route("/api/photos/:id", get(crate::photos::get::handler))
.route("/api/photos/:id/thumb/:size", get(crate::photos::serve::thumb))
```

### Task 3.4 — Integration test

`backend/tests/photos.rs`: full flow — start MinIO container, run a real
testcontainers Postgres, signup → upload a synthetic JPEG → poll until
status='ready' → GET detail → GET thumb → assert bytes look like JPEG.

Skip if writing the multipart body manually proves too painful; a curl
manual smoke test (in Batch 5) covers correctness.

### Quality gate

```bash
just check
cargo test --manifest-path backend/Cargo.toml
```

### Commits

1. `feat(backend/photos): add photo queries`
2. `feat(backend/photos): add upload pipeline (POST /api/photos)`
3. `feat(backend/photos): add detail + thumbnail serve + list endpoints`
4. `test(backend/photos): integration test for upload flow` (optional)

---

## Batch 4 — Frontend wiring + /upload UI

### Task 4.1 — Update `Photo.svelte` to support real images

Add a `src?: string` prop that, when present, renders an `<img>`
overlaid on the gradient. The gradient acts as a low-quality preview
fallback while the real image loads.

### Task 4.2 — `/upload` route

Port `ScreenUpload` from `docs/design/handoff/screens-2.jsx`:
- `frontend/src/routes/upload/+page.svelte` — 3-step stepper UI
- `frontend/src/routes/upload/+page.server.ts` — form action POSTs
  multipart to `${API}/api/photos`, redirects to `/photo/<id>` on
  success, returns `fail()` on errors
- Require auth (redirect to `/signin` if `locals.user` is null)

### Task 4.3 — Wire gallery + profile to real photos

- `frontend/src/routes/+page.server.ts` — fetch `${API}/api/photos`;
  fall back to placeholder data when response is empty (so the unauthenticated landing keeps demo content).
- `frontend/src/routes/u/[username]/+page.server.ts` — when username is
  a UUID-looking string, fetch real photos for that owner; otherwise
  keep the placeholder Marie Dubois route.
- Each gallery thumbnail uses `<Photo src="${API}/api/photos/${id}/thumb/400" target={p.target} />`.

### Task 4.4 — Wire photo detail page

- `frontend/src/routes/photo/[slug]/+page.server.ts` — when slug looks
  like a UUID, call `${API}/api/photos/${id}` and serve the real
  metadata + the 1200px thumb URL. Otherwise keep the placeholder
  NGC 7000 route.

### Quality gate

```bash
pnpm -C frontend check && pnpm -C frontend lint && pnpm -C frontend build
just check
```

### Commits

1. `feat(frontend): Photo component supports real images`
2. `feat(frontend): add /upload route (3-step stepper, multipart POST)`
3. `feat(frontend): gallery + profile load real photos with placeholder fallback`
4. `feat(frontend): photo detail page serves real photos by UUID`

---

## Batch 5 — Browser smoke test + merge

Manual smoke:
1. `just dev` (postgres, minio, backend, frontend)
2. Sign up via /signup
3. Click avatar → /upload → drag a JPEG → submit
4. Wait ~2s, refresh /
5. See the new photo at the top of the gallery
6. Click → /photo/<id> shows real image + EXIF
7. Click "Marie" avatar in detail → /u/<id> shows the photo

Then merge:
```bash
git checkout main
git merge --no-ff feat/phase-5-photos
git tag -a v0.4.0-photos -m "Astrophoto v0.4.0 — upload pipeline"
git branch -d feat/phase-5-photos
```

## Self-review (post-write)

- All endpoints require auth where appropriate (`POST /api/photos` does;
  `GET` endpoints are public per the design).
- All blocking work in `spawn_blocking` (image decode, EXIF parse).
- All sqlx macros cached in `.sqlx/`.
- No `.unwrap()` in lib code outside the controlled `parse()` calls on
  HeaderValue.
- The `Photo` placeholder gradient remains for unauthenticated demo /
  fallback during image load.
