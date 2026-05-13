use axum::{Json, extract::State};

use crate::AppError;
use crate::api_types::{BatchApplyRequest, BatchApplyResponse};
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<BatchApplyRequest>,
) -> Result<Json<BatchApplyResponse>, AppError> {
    if body.ids.is_empty() {
        return Err(AppError::bad_request("ids must not be empty"));
    }
    if body.ids.len() > 50 {
        return Err(AppError::bad_request("too many ids"));
    }
    if let Some(t) = &body.target
        && t.is_empty()
    {
        return Err(AppError::bad_request("target cannot be empty string"));
    }
    if let Some(tags) = &body.tags
        && tags.len() > 8
    {
        return Err(AppError::bad_request("max 8 tags"));
    }

    let owners = sqlx::query!(
        r#"select id, owner_id, published_at from photos where id = any($1)"#,
        &body.ids
    )
    .fetch_all(&state.pool)
    .await?;

    if owners.len() != body.ids.len() {
        return Err(AppError::not_found("one or more photo ids do not exist"));
    }

    for r in &owners {
        if r.owner_id != user.id {
            return Err(AppError::Forbidden);
        }
    }
    for r in &owners {
        if r.published_at.is_some() {
            return Err(AppError::bad_request(
                "one or more ids refer to published photos",
            ));
        }
    }

    if let Some(target) = &body.target {
        let mut tx = state.pool.begin().await?;
        sqlx::query!(
            "update photos set target = $1 where id = any($2)",
            target,
            &body.ids
        )
        .execute(&mut *tx)
        .await?;

        for id in &body.ids {
            crate::photos::targets::attach_primary_by_freetext(&mut tx, *id, target).await?;
        }
        tx.commit().await?;
    }

    if let Some(tags) = &body.tags {
        sqlx::query!("delete from photo_tags where photo_id = any($1)", &body.ids)
            .execute(&state.pool)
            .await?;
        if !tags.is_empty() {
            for id in &body.ids {
                crate::photos::tags::attach(&state.pool, *id, tags).await?;
            }
        }
    }

    Ok(Json(BatchApplyResponse {
        applied: body.ids.len() as u32,
    }))
}
