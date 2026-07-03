use axum::{Json, extract::State};

use crate::AppError;
use crate::api_types::{
    BatchPublishRequest, BatchPublishResponse, PublishedItem, SkipReason, SkippedItem,
};
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<BatchPublishRequest>,
) -> Result<Json<BatchPublishResponse>, AppError> {
    if body.ids.is_empty() {
        return Err(AppError::bad_request("ids must not be empty"));
    }
    if body.ids.len() > 50 {
        return Err(AppError::bad_request("too many ids"));
    }

    // Dedup before the existence check: `= any($1)` deduplicates on the
    // SQL side, so a repeated (existing, owned) id used to make
    // rows.len() < ids.len() and fail the whole batch with a misleading
    // 404 — a double-click was enough to trigger it.
    let mut ids = body.ids.clone();
    ids.sort_unstable();
    ids.dedup();

    let mut tx = state.pool.begin().await?;

    let rows = sqlx::query!(
        "select id, owner_id, status, published_at, short_id from photos where id = any($1)",
        &ids
    )
    .fetch_all(&mut *tx)
    .await?;

    if rows.len() != ids.len() {
        return Err(AppError::not_found("one or more photo ids do not exist"));
    }

    for r in &rows {
        if r.owner_id != user.id {
            return Err(AppError::Forbidden);
        }
    }

    let mut published = Vec::new();
    let mut skipped = Vec::new();

    for r in &rows {
        let reason = if r.published_at.is_some() {
            Some(SkipReason::AlreadyPublished)
        } else if r.status == "failed" {
            Some(SkipReason::Failed)
        } else if r.status != "ready" {
            Some(SkipReason::StillProcessing)
        } else {
            None
        };

        match reason {
            Some(reason) => skipped.push(SkippedItem {
                id: r.id.to_string(),
                reason,
            }),
            None => {
                // Guarded like the single-photo publish: a replace claim
                // racing this transaction must not publish a mid-swap
                // photo, and published_at is written at most once. (No
                // last_step='caption' — that step was removed in 56acf4e.)
                let n = sqlx::query!(
                    "update photos set published_at = now()
                      where id = $1 and status = 'ready' and published_at is null",
                    r.id
                )
                .execute(&mut *tx)
                .await?
                .rows_affected();
                if n == 0 {
                    skipped.push(SkippedItem {
                        id: r.id.to_string(),
                        reason: SkipReason::StillProcessing,
                    });
                } else {
                    published.push(PublishedItem {
                        id: r.id.to_string(),
                        short_id: r.short_id.clone(),
                    });
                }
            }
        }
    }

    tx.commit().await?;

    Ok(Json(BatchPublishResponse { published, skipped }))
}
