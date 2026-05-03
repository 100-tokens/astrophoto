use axum::{Json, extract::State, response::IntoResponse};
use serde::{Deserialize, Serialize};
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

const FREE_MAX: u64 = 50 * 1024 * 1024;
const SUBSCRIBER_MAX: u64 = 200 * 1024 * 1024;
const PUT_TTL_SECS: u64 = 600;

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<InitBody>,
) -> Result<impl IntoResponse, AppError> {
    if body.files.is_empty() || body.files.len() > 10 {
        return Err(AppError::Validation("files must be 1..=10".into()));
    }

    let tier: String = sqlx::query_scalar!("select tier from users where id = $1", user.id)
        .fetch_one(&state.pool)
        .await?;

    let max_bytes = if tier == "subscriber" {
        SUBSCRIBER_MAX
    } else {
        FREE_MAX
    };

    // Pre-validation pass: check size and MIME before touching the database.
    for f in &body.files {
        if f.size > max_bytes {
            return Err(AppError::QuotaExceeded(format!(
                "file {} exceeds {} bytes",
                f.name, max_bytes
            )));
        }
        match f.mime.as_str() {
            "image/jpeg" | "image/png" | "image/tiff" => {}
            _ => return Err(AppError::UnsupportedFormat(f.mime.clone())),
        }
    }

    let mut out = Vec::with_capacity(body.files.len());
    let mut tx = state.pool.begin().await?;

    for f in body.files {
        // Per-owner hash dedup: same owner may not re-upload the same file.
        let dup: Option<Uuid> = sqlx::query_scalar!(
            "select id from photos where owner_id = $1 and original_hash = $2",
            user.id,
            f.hash
        )
        .fetch_optional(&mut *tx)
        .await?;
        if dup.is_some() {
            return Err(AppError::Conflict("file already uploaded".into()));
        }

        // Insert a pending row with short_id collision retry (up to 5 attempts).
        let mut attempts = 0u8;
        let (photo_id, short, key) = loop {
            attempts += 1;
            let pid = Uuid::new_v4();
            let s = short_id::generate();
            let k = format!("originals/{pid}");
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
            .execute(&mut *tx)
            .await
            {
                Ok(_) => break (pid, s, k),
                Err(sqlx::Error::Database(ref db_err))
                    if db_err.constraint() == Some("photos_short_id_uidx") && attempts < 5 =>
                {
                    continue;
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
