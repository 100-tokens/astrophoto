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
use crate::photos::{magic, pipeline, platesolve_upload, queries};

const MAX_BYTES: usize = 50 * 1024 * 1024;
const ALLOWED_MIMES: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/tiff",
    "application/x-xisf",
];

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let row = sqlx::query!("select owner_id from photos where id = $1", id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::not_found("photo"))?;
    if row.owner_id != user.id {
        return Err(AppError::Forbidden);
    }

    let mut file_bytes: Option<Bytes> = None;
    let mut filename = String::from("upload.bin");
    let mut mime = String::from("application/octet-stream");
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::Validation(format!("multipart: {e}")))?
    {
        if field.name() == Some("file") {
            if let Some(n) = field.file_name() {
                filename = n.to_string();
            }
            if let Some(c) = field.content_type() {
                mime = c.to_string();
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
    }
    let bytes = file_bytes.ok_or_else(|| AppError::Validation("missing file".into()))?;
    if !ALLOWED_MIMES.contains(&mime.as_str()) {
        return Err(AppError::Validation(format!("unsupported mime: {mime}")));
    }
    // XISF can't be decoded locally — it goes through the external
    // plate-solve service (same as upload_finalize). Without a client
    // configured the photo would brick in `awaiting-calibration`.
    let is_xisf = mime == "application/x-xisf";
    if is_xisf && state.platesolve.is_none() {
        return Err(AppError::UnsupportedFormat(format!(
            "{mime} (plate-solve service not configured)"
        )));
    }

    // Magic-byte sniff BEFORE any destructive prep. The steps below
    // enqueue the old master for deletion and swap the storage key, so
    // a payload that can never finalize must be rejected up front —
    // otherwise the photo is left failed and the 7-day sweep would
    // destroy the only good original.
    let sig = magic::sniff(&bytes);
    if !magic::matches_mime(sig, &mime) {
        return Err(AppError::MagicByteMismatch(format!("{sig:?}")));
    }

    // Atomically claim the pipeline. The previous read-then-check on
    // `status` raced concurrent replaces: both passed the check, both
    // uploaded a fresh original, and the loser's key was referenced by
    // nothing — leaking in S3 forever. The claim UPDATE flips status to
    // 'processing' and returns the about-to-be-replaced storage key in
    // one statement; a concurrent replace (or finalize) loses the claim
    // and bounces here. Claimed only AFTER body validation, so a
    // malformed payload never touches the row.
    let Some(old_storage_key) = queries::claim_for_replace(&state.pool, id).await? else {
        return Err(AppError::BadRequest("pipeline busy".into()));
    };

    let new_key = format!("originals/{}", Uuid::new_v4());
    let prep: Result<(), AppError> = async {
        // 1. Upload the new master FIRST — a pure addition. If it fails,
        //    nothing has been queued or swapped and mark_failed below
        //    leaves a cleanly retryable photo (the orphan S3 object is
        //    harmless).
        state.storage.put(&new_key, &mime, bytes.clone()).await?;

        // 2. One transaction: queue the old keys for deferred deletion,
        //    swap the storage key, drop the old thumbnail rows. Atomic on
        //    purpose — a key must NEVER sit in photo_pending_deletes
        //    while it is still the photo's live asset. (A failed prep
        //    used to leave the live master queued; a later re-finalize
        //    flipped the photo back to 'ready' and the 7-day sweep then
        //    destroyed its only original.)
        let mut tx = state.pool.begin().await?;
        let mut to_stash = vec![old_storage_key.clone()];
        let old_thumb_keys: Vec<String> =
            sqlx::query_scalar!("select storage_key from thumbnails where photo_id = $1", id)
                .fetch_all(&mut *tx)
                .await?;
        to_stash.extend(old_thumb_keys);
        queries::enqueue_pending_deletes(&mut tx, id, &to_stash).await?;
        queries::swap_storage_key_for_replace(
            &mut tx,
            id,
            &new_key,
            &filename,
            &mime,
            bytes.len() as i64,
        )
        .await?;
        sqlx::query!("delete from thumbnails where photo_id = $1", id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
    .await;
    if let Err(e) = prep {
        // The claim flipped status to 'processing'; record a terminal
        // failure so the photo doesn't sit stuck and a retry can re-claim.
        // The transaction above rolled back, so no live key is queued.
        let reason = format!("replace prep: {e}");
        let _ = queries::mark_failed(&state.pool, id, &reason).await;
        return Err(e);
    }

    // 5a. XISF: no local decoder — route through the auto-calibrate
    // path exactly like upload_finalize. The solve service returns the
    // WCS + display JPEG and transitions status to ready/failed. The
    // stashed pending deletes stay queued; the purge sweep only drains
    // rows for photos back in status='ready', so the old original
    // survives until the replacement is actually viable.
    if is_xisf {
        queries::mark_awaiting_calibration(&state.pool, id).await?;
        platesolve_upload::auto_calibrate_xisf(state.clone(), id, new_key, user.id);
        return Ok(StatusCode::ACCEPTED);
    }

    // 5b. Spawn pipeline with Replace options — drains pending deletes on success.
    let pool = state.pool.clone();
    let storage = state.storage.clone();
    tokio::spawn(async move {
        if let Err(e) = pipeline::finalize(
            &pool,
            storage,
            id,
            bytes,
            pipeline::PipelineOptions::Replace,
        )
        .await
        {
            let reason = format!("{e}");
            tracing::error!(photo_id=%id, error=%reason, "replace finalize failed");
            let _ = queries::mark_failed(&pool, id, &reason).await;
        }
    });

    Ok(StatusCode::ACCEPTED)
}
