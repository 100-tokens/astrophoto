use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::AppError;

pub struct PhotoRow {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub short_id: String,
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
    pub aperture_f: Option<f32>,
    pub gain: Option<i16>,
    pub sensor_temp_c: Option<f32>,
    pub sessions: Option<i16>,
    pub ra_deg: Option<f64>,
    pub dec_deg: Option<f64>,
    pub target: Option<String>,
    pub caption: Option<String>,
    // Migration 0009: user-entered acquisition fields (no EXIF source)
    // and the small fixed-taxonomy `category` column.
    pub scope: Option<String>,
    pub mount: Option<String>,
    pub guiding: Option<String>,
    pub category: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub replaced_at: Option<DateTime<Utc>>,
    pub original_uploaded_at: DateTime<Utc>,
    pub last_step: Option<String>,
    pub pipeline_error: Option<String>,
    // Migration 0014: equipment setup link + per-photo focal modifier.
    pub setup_id: Option<uuid::Uuid>,
    pub focal_modifier: Option<String>,
    // Legacy filter cache — comma-joined display names from photo_filters.
    pub filters: Option<String>,
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
    for _ in 0..5 {
        let short_id = crate::photos::short_id::generate();
        let res = sqlx::query!(
            r#"
            insert into photos (owner_id, storage_key, original_name, bytes, mime,
                                target, caption, short_id, status, last_step, original_uploaded_at)
            values ($1, $2, $3, $4, $5, $6, $7, $8, 'processing', 'upload', now())
            returning id
            "#,
            owner_id,
            storage_key,
            original_name,
            bytes,
            mime,
            target,
            caption,
            short_id,
        )
        .fetch_one(pool)
        .await;
        match res {
            Ok(row) => return Ok(row.id),
            Err(sqlx::Error::Database(ref db_err))
                if db_err.constraint() == Some("photos_short_id_uidx") =>
            {
                continue;
            }
            Err(e) => return Err(AppError::Database(e)),
        }
    }
    Err(AppError::Internal(
        "short_id collision retries exhausted".into(),
    ))
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
            aperture_f = coalesce($10, aperture_f),
            exif_json=$11
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
        exif.aperture_f,
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

/// Mark an XISF upload as awaiting calibration: astrophoto has no XISF
/// decoder, so the standard pipeline (EXIF / thumbnails / display master
/// / blurhash) is skipped. The auto-platesolve trigger picks the photo
/// up next and fills in `display_key` + telemetry via the external
/// service, then transitions status to `ready`.
pub async fn mark_awaiting_calibration(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    // Runtime query — the cached `.sqlx/` doesn't have this exact SQL
    // yet. Promoted to `sqlx::query!` after `cargo sqlx prepare`.
    sqlx::query("update photos set status='awaiting-calibration', pipeline_error=null where id=$1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(AppError::from)?;
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
        select id, owner_id, short_id, storage_key, original_name, bytes, mime,
               width, height, taken_at, camera, lens, iso, exposure_s, focal_mm,
               aperture_f, gain, sensor_temp_c, sessions, ra_deg, dec_deg,
               target, caption, scope, mount, guiding, category, status, created_at,
               published_at, replaced_at, original_uploaded_at, last_step, pipeline_error,
               setup_id, focal_modifier, filters
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
        select id, owner_id, short_id, storage_key, original_name, bytes, mime,
               width, height, taken_at, camera, lens, iso, exposure_s, focal_mm,
               aperture_f, gain, sensor_temp_c, sessions, ra_deg, dec_deg,
               target, caption, scope, mount, guiding, category, status, created_at,
               published_at, replaced_at, original_uploaded_at, last_step, pipeline_error,
               setup_id, focal_modifier, filters
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
        select id, owner_id, short_id, storage_key, original_name, bytes, mime,
               width, height, taken_at, camera, lens, iso, exposure_s, focal_mm,
               aperture_f, gain, sensor_temp_c, sessions, ra_deg, dec_deg,
               target, caption, scope, mount, guiding, category, status, created_at,
               published_at, replaced_at, original_uploaded_at, last_step, pipeline_error,
               setup_id, focal_modifier, filters
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
        select p.id, p.owner_id, p.short_id, p.storage_key, p.original_name, p.bytes, p.mime,
               p.width, p.height, p.taken_at, p.camera, p.lens, p.iso,
               p.exposure_s, p.focal_mm,
               p.aperture_f, p.gain, p.sensor_temp_c, p.sessions, p.ra_deg, p.dec_deg,
               p.target, p.caption, p.scope, p.mount, p.guiding, p.category,
               p.status, p.created_at,
               p.published_at, p.replaced_at, p.original_uploaded_at, p.last_step, p.pipeline_error,
               p.setup_id, p.focal_modifier, p.filters
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

/// List published photos that reference the given equipment item via
/// the `photo_filters` junction. Used by the catalog item-detail page
/// for `kind = 'filter'` so the "photos using this filter" grid can
/// be sourced authoritatively (the legacy `photos.filters` text cache
/// is brittle for that lookup — see `gotchas` in CLAUDE.md).
pub async fn list_by_filter_item(
    pool: &PgPool,
    filter_item_id: Uuid,
    limit: i64,
) -> Result<Vec<PhotoRow>, AppError> {
    // `photo_filters.(photo_id, item_id)` is the PK so the inner join
    // produces at most one row per photo for a given item_id — no
    // DISTINCT needed.
    let rows = sqlx::query_as!(
        PhotoRow,
        r#"
        select p.id, p.owner_id, p.short_id, p.storage_key, p.original_name, p.bytes, p.mime,
               p.width, p.height, p.taken_at, p.camera, p.lens, p.iso, p.exposure_s, p.focal_mm,
               p.aperture_f, p.gain, p.sensor_temp_c, p.sessions, p.ra_deg, p.dec_deg,
               p.target, p.caption, p.scope, p.mount, p.guiding, p.category, p.status, p.created_at,
               p.published_at, p.replaced_at, p.original_uploaded_at, p.last_step, p.pipeline_error,
               p.setup_id, p.focal_modifier, p.filters
        from photos p
        join photo_filters pf on pf.photo_id = p.id
        where pf.item_id = $1 and p.published_at is not null
        order by p.published_at desc, p.id desc
        limit $2
        "#,
        filter_item_id,
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
        select id, owner_id, short_id, storage_key, original_name, bytes, mime,
               width, height, taken_at, camera, lens, iso, exposure_s, focal_mm,
               aperture_f, gain, sensor_temp_c, sessions, ra_deg, dec_deg,
               target, caption, scope, mount, guiding, category, status, created_at,
               published_at, replaced_at, original_uploaded_at, last_step, pipeline_error,
               setup_id, focal_modifier, filters
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
