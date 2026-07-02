use axum::{Json, extract::State, response::IntoResponse};
use serde::{Deserialize, Serialize};
// For Transaction::begin (savepoints) in the short_id retry loop.
use sqlx::Acquire;
use uuid::Uuid;

use crate::auth::middleware::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;
use crate::photos::short_id;

#[derive(Deserialize)]
pub struct File {
    pub name: String,
    pub size: u64,
    pub mime: String,
    pub hash: String,
}

#[derive(Deserialize)]
pub struct InitBody {
    pub files: Vec<File>,
}

#[derive(Serialize)]
pub struct InitFile {
    pub photo_id: String,
    pub short_id: String,
    pub presigned_put_url: String,
}

#[derive(Serialize)]
pub struct InitResponse {
    pub files: Vec<InitFile>,
}

const PUT_TTL_SECS: u64 = 600;

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<InitBody>,
) -> Result<impl IntoResponse, AppError> {
    if body.files.is_empty() || body.files.len() > 12 {
        return Err(AppError::Validation("files must be 1..=12".into()));
    }

    let tier: String = sqlx::query_scalar!("select tier from users where id = $1", user.id)
        .fetch_one(&state.pool)
        .await?;

    // Per-tier upload ceiling comes from the runtime app settings (super-admin
    // editable); falls back to the historical defaults (50/200 MiB) on any
    // settings read error.
    let max_bytes = crate::settings::get(&state.pool)
        .await
        .upload_max_bytes(&tier);

    // Pre-validation pass: check size and MIME before touching the database.
    for f in &body.files {
        if f.size > max_bytes {
            return Err(AppError::QuotaExceeded(format!(
                "file {} exceeds {} bytes",
                f.name, max_bytes
            )));
        }
        match f.mime.as_str() {
            // Standard formats: decoded inline by the upload pipeline
            // (EXIF, thumbnails, display master, blurhash).
            "image/jpeg" | "image/png" | "image/tiff" => {}
            // XISF: NOT decoded inline (no XISF decoder in astrophoto).
            // Pipeline auto-triggers plate-solve on the external service
            // which returns the WCS + a display JPEG + structured FITS/PCL
            // metadata. Status stays `awaiting-calibration` until that
            // round-trip completes. The platesolve client must be
            // configured on this deployment — otherwise the photo would
            // get stuck `awaiting-calibration` forever.
            "application/x-xisf" => {
                if state.platesolve.is_none() {
                    return Err(AppError::UnsupportedFormat(format!(
                        "{} (plate-solve service not configured)",
                        f.mime
                    )));
                }
            }
            _ => return Err(AppError::UnsupportedFormat(f.mime.clone())),
        }
    }

    let mut out = Vec::with_capacity(body.files.len());
    let mut tx = state.pool.begin().await?;

    for f in body.files {
        // Per-owner hash dedup: same owner may not re-upload the same file.
        // Failed rows don't count (mirrors the partial unique index,
        // migration 0039): a finalize-stage failure must not block
        // retrying the identical bytes.
        let dup: Option<Uuid> = sqlx::query_scalar!(
            "select id from photos \
             where owner_id = $1 and original_hash = $2 and status <> 'failed'",
            user.id,
            f.hash
        )
        .fetch_optional(&mut *tx)
        .await?;
        if dup.is_some() {
            return Err(AppError::Conflict("file already uploaded".into()));
        }

        // Insert a pending row with short_id collision retry (up to 5
        // attempts). Each attempt runs in a savepoint: a unique-violation
        // otherwise poisons the whole outer transaction (Postgres 25P02
        // "current transaction is aborted") and no retry INSERT could
        // ever succeed — the loop was dead code turning a recoverable
        // collision into a 500.
        let mut attempts = 0u8;
        let (photo_id, short, key) = loop {
            attempts += 1;
            let pid = Uuid::new_v4();
            let s = short_id::generate();
            let k = format!("originals/{pid}");
            let mut sp = tx.begin().await?;
            match sqlx::query!(
                r#"
                insert into photos
                    (id, owner_id, storage_key, original_name, bytes, mime,
                     original_hash, short_id, status, last_step, original_uploaded_at)
                values ($1, $2, $3, $4, $5, $6, $7, $8, 'pending', 'upload', now())
                "#,
                pid,
                user.id,
                k,
                f.name,
                f.size as i64,
                f.mime,
                f.hash,
                s
            )
            .execute(&mut *sp)
            .await
            {
                Ok(_) => {
                    sp.commit().await?;
                    break (pid, s, k);
                }
                Err(sqlx::Error::Database(ref db_err))
                    if db_err.constraint() == Some("photos_short_id_uidx") && attempts < 5 =>
                {
                    sp.rollback().await?;
                    continue;
                }
                // Concurrent double-submit of the same file: the
                // sequential pre-check above missed it, but the partial
                // unique index catches it. Same 409 as the pre-check.
                Err(sqlx::Error::Database(ref db_err))
                    if db_err.constraint() == Some("photos_owner_hash_uidx") =>
                {
                    return Err(AppError::Conflict("file already uploaded".into()));
                }
                Err(e) => return Err(AppError::Database(e)),
            }
        };

        let url = state
            .storage
            .presigned_put(&key, &f.mime, f.size, PUT_TTL_SECS)
            .await?;

        out.push(InitFile {
            photo_id: photo_id.to_string(),
            short_id: short,
            presigned_put_url: url,
        });
    }

    tx.commit().await?;

    Ok(Json(InitResponse { files: out }))
}
