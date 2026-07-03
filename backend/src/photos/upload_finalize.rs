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
use crate::photos::{magic, pipeline, platesolve_upload, queries};

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

    // Atomically claim the row BEFORE touching storage. Three jobs:
    // (a) the hourly reaper only deletes status='pending' rows, so an
    //     in-flight finalize can no longer be reaped (row + S3 original
    //     destroyed) if it straddles the 24h mark or crashes mid-run —
    //     and the claim must precede the S3 reads or the reaper can
    //     race exactly that window;
    // (b) a concurrent duplicate finalize loses the claim and bounces
    //     with 409 instead of buffering the original twice and running
    //     the pipeline twice;
    // (c) the magic-mismatch mark_failed below can't clobber a pipeline
    //     another finalize already owns.
    let claimed = sqlx::query!(
        "update photos set status='processing'
          where id = $1 and status in ('pending', 'failed')",
        id
    )
    .execute(&state.pool)
    .await?
    .rows_affected();
    if claimed == 0 {
        return Err(AppError::Conflict("finalize already in progress".into()));
    }

    // Magic-byte sniff over a 16-byte range GET — never the whole object.
    // `None` means the client's PUT never arrived (or the presigned URL
    // expired): release the claim back to 'pending' so a later retry can
    // finalize once the bytes exist, and tell the caller to redo the PUT.
    let head = match state.storage.get_range(&row.storage_key, 0, 15).await? {
        Some(h) => h,
        None => {
            sqlx::query!(
                "update photos set status='pending' where id = $1 and status='processing'",
                id
            )
            .execute(&state.pool)
            .await?;
            return Err(AppError::PendingFinalizeStuck(
                "no object at storage_key — did the PUT succeed?".into(),
            ));
        }
    };
    let sig = magic::sniff(&head);
    if !magic::matches_mime(sig, &row.mime) {
        queries::mark_failed(&state.pool, id, "magic-byte mismatch").await?;
        return Err(AppError::MagicByteMismatch(format!("{sig:?}")));
    }

    // XISF takes a different path: astrophoto has no XISF decoder, so
    // the standard EXIF / thumbnail / display-master / blurhash pipeline
    // can't run. We mark the photo `awaiting-calibration`, fire the
    // auto-platesolve trigger (background task — fetches the original
    // from S3 under its own semaphore, forwards to the plate-solve
    // service with `render=true`, persists the returned JPEG as the
    // display master, transitions status to `ready` or `failed`), and
    // return 200 immediately. The finalize path itself never buffers
    // the (potentially huge) XISF.
    if row.mime == "application/x-xisf" {
        queries::mark_awaiting_calibration(&state.pool, id).await?;
        platesolve_upload::auto_calibrate_xisf(
            state.clone(),
            id,
            row.storage_key.clone(),
            row.owner_id,
        );
        return Ok(Json(FinalizeResp {
            status: "awaiting-calibration".into(),
            display_key: None,
        }));
    }

    // Bound how many finalizes may hold an original (+ its decoded
    // image) in memory at once — acquired BEFORE the full-object GET.
    // Unbounded, a 12-file batch of tier-max originals OOMs the small
    // Koyeb instance. Queued waiters hold only this claim, no bytes.
    let _permit = state
        .finalize_permits
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| AppError::Internal("finalize semaphore closed".into()))?;

    // Fetch the full object. The range GET above proved it existed, but
    // it may vanish mid-queue (cancel/delete race) — same recovery as
    // the missing-object case.
    let bytes = match state.storage.get(&row.storage_key).await? {
        Some(b) => b,
        None => {
            sqlx::query!(
                "update photos set status='pending' where id = $1 and status='processing'",
                id
            )
            .execute(&state.pool)
            .await?;
            return Err(AppError::PendingFinalizeStuck(
                "no object at storage_key — did the PUT succeed?".into(),
            ));
        }
    };

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
