use chrono::{DateTime, Utc};
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
    pub published_at: Option<DateTime<Utc>>,
    pub replaced_at: Option<DateTime<Utc>>,
    pub original_uploaded_at: DateTime<Utc>,
    pub last_step: Option<String>,
    pub pipeline_error: Option<String>,
}

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
        owner_id,
        storage_key,
        original_name,
        bytes,
        mime,
        target,
        caption,
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
            status='ready', pipeline_error=null,
            width=$2, height=$3,
            taken_at=$4, camera=$5, lens=$6, iso=$7,
            exposure_s=$8, focal_mm=$9,
            exif_json=$10
        where id=$1
        "#,
        id,
        width,
        height,
        exif.taken_at,
        exif.camera,
        exif.lens,
        exif.iso,
        exif.exposure_s,
        exif.focal_mm,
        exif.raw,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_ready_size_only(
    pool: &PgPool,
    id: Uuid,
    width: i32,
    height: i32,
) -> Result<(), AppError> {
    sqlx::query!(
        "update photos set status='ready', width=$2, height=$3, pipeline_error=null where id=$1",
        id,
        width,
        height
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_display_metadata(
    pool: &PgPool,
    id: Uuid,
    display_key: &str,
    blurhash: &str,
) -> Result<(), AppError> {
    sqlx::query!(
        "update photos set display_key = $2, blurhash = $3 where id = $1",
        id,
        display_key,
        blurhash,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_failed(pool: &PgPool, id: Uuid, reason: &str) -> Result<(), AppError> {
    sqlx::query!(
        "update photos set status='failed', pipeline_error=$2 where id=$1",
        id,
        reason
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn pending_deletes_for(pool: &PgPool, photo_id: Uuid) -> Result<Vec<String>, AppError> {
    let rows = sqlx::query_scalar!(
        "select storage_key from photo_pending_deletes where photo_id = $1",
        photo_id
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn drain_pending_deletes(pool: &PgPool, photo_id: Uuid) -> Result<(), AppError> {
    sqlx::query!(
        "delete from photo_pending_deletes where photo_id = $1",
        photo_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn enqueue_pending_deletes(
    pool: &PgPool,
    photo_id: Uuid,
    storage_keys: &[String],
) -> Result<(), AppError> {
    if storage_keys.is_empty() {
        return Ok(());
    }
    for key in storage_keys {
        sqlx::query!(
            "insert into photo_pending_deletes (photo_id, storage_key) values ($1, $2)",
            photo_id,
            key
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn swap_storage_key_for_replace(
    pool: &PgPool,
    id: Uuid,
    new_key: &str,
    original_name: &str,
    mime: &str,
    bytes: i64,
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
        id,
        new_key,
        original_name,
        mime,
        bytes
    )
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
        photo_id,
        size,
        storage_key,
        bytes,
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

pub async fn list_by_owner(
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
        where owner_id = $1 and published_at is not null
        order by published_at desc
        limit $2
        "#,
        owner_id,
        limit
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
               target, caption, status, created_at,
               published_at, replaced_at, original_uploaded_at, last_step, pipeline_error
        from photos
        where published_at is not null
        order by published_at desc
        limit $1
        "#,
        limit
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_following(
    pool: &PgPool,
    follower_id: Uuid,
    limit: i64,
) -> Result<Vec<PhotoRow>, AppError> {
    let rows = sqlx::query_as!(
        PhotoRow,
        r#"
        select p.id, p.owner_id, p.storage_key, p.original_name, p.bytes, p.mime,
               p.width, p.height, p.taken_at, p.camera, p.lens, p.iso,
               p.exposure_s, p.focal_mm, p.target, p.caption, p.status, p.created_at,
               p.published_at, p.replaced_at, p.original_uploaded_at, p.last_step, p.pipeline_error
        from photos p
        join follows f on f.followed_id = p.owner_id
        where f.follower_id = $1 and p.published_at is not null
        order by p.published_at desc
        limit $2
        "#,
        follower_id,
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
        photo_id,
        size
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.storage_key))
}

pub async fn count_by_owner(pool: &PgPool, owner_id: Uuid) -> Result<i64, AppError> {
    let row = sqlx::query!(
        r#"select count(*) as "count!" from photos where owner_id = $1 and published_at is not null"#,
        owner_id
    )
    .fetch_one(pool)
    .await?;
    Ok(row.count)
}

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
        owner_id,
        limit
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

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
