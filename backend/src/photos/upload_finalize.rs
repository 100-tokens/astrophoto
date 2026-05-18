use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::middleware::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;
use crate::photos::{magic, pipeline, queries};

#[derive(Serialize)]
pub struct FinalizeResp {
    pub status: String,
    pub display_key: Option<String>,
}

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let row = sqlx::query!(
        r#"select owner_id as "owner_id!", storage_key, mime, status, display_key
           from photos where id = $1"#,
        id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("photo".into()))?;

    if row.owner_id != user.id {
        return Err(AppError::NotFound("photo".into()));
    }

    // Idempotency: if already ready, return current state without re-running.
    if row.status == "ready" {
        return Ok(Json(FinalizeResp {
            status: "ready".into(),
            display_key: row.display_key,
        }));
    }

    // Fetch the full object from storage. `None` means the client's PUT never
    // arrived (or the presigned URL expired). Caller must redo init + PUT.
    let bytes = state.storage.get(&row.storage_key).await?.ok_or_else(|| {
        AppError::PendingFinalizeStuck("no object at storage_key — did the PUT succeed?".into())
    })?;

    // Magic-byte sniff over the first 16 bytes.
    let head: Vec<u8> = bytes.iter().take(16).cloned().collect();
    let sig = magic::sniff(&head);
    if !magic::matches_mime(sig, &row.mime) {
        queries::mark_failed(&state.pool, id, "magic-byte mismatch").await?;
        return Err(AppError::MagicByteMismatch(format!("{sig:?}")));
    }

    // XISF takes a different path: astrophoto has no XISF decoder, so
    // the standard EXIF / thumbnail / display-master / blurhash pipeline
    // can't run. We mark the photo `awaiting-calibration` and hand off
    // to the auto-platesolve trigger (separate background task) which
    // fetches the original from S3, forwards it to the plate-solve
    // service with `render=true`, persists the returned JPEG as the
    // display master, then transitions status to `ready`.
    if row.mime == "application/x-xisf" {
        queries::mark_awaiting_calibration(&state.pool, id).await?;
        return Ok(Json(FinalizeResp {
            status: "awaiting-calibration".into(),
            display_key: None,
        }));
    }

    // Run the full pipeline (EXIF + thumbnails + display master + blurhash).
    // On error, mark the photo failed so the caller knows it needs to re-init.
    if let Err(e) = pipeline::finalize(
        &state.pool,
        Arc::clone(&state.storage),
        id,
        bytes,
        pipeline::PipelineOptions::Initial,
    )
    .await
    {
        let reason = format!("{e}");
        let _ = queries::mark_failed(&state.pool, id, &reason).await;
        return Err(e);
    }

    let display_key: Option<String> =
        sqlx::query_scalar!("select display_key from photos where id = $1", id)
            .fetch_one(&state.pool)
            .await?;

    Ok(Json(FinalizeResp {
        status: "ready".into(),
        display_key,
    }))
}
