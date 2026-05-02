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
