//! POST /api/me/handle — rename the authenticated user's handle.
//!
//! Validates the new handle, then inside a transaction:
//!   1. Reads the current handle.
//!   2. Short-circuits with 204 No Content if the handle is unchanged.
//!   3. Updates `users.handle` (constraint violation → 409 Conflict).
//!   4. Inserts (or refreshes) a row in `handle_redirects` so old paths
//!      can 301 to the new handle. The old handle becomes reservable again
//!      after a 90-day cooldown (`released_at`).

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

use crate::auth::handle::{HandleError, validate};
use crate::auth::middleware::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;

#[derive(Deserialize)]
pub struct Body {
    pub handle: String,
}

pub async fn rename(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<Body>,
) -> Result<impl IntoResponse, AppError> {
    validate(&body.handle).map_err(|e| match e {
        HandleError::Format => AppError::Validation("invalid handle format".into()),
        HandleError::Reserved => AppError::Conflict("handle is reserved".into()),
    })?;

    let mut tx = state.pool.begin().await?;

    // Read current handle for the redirect row. Must be inside the
    // transaction so we don't race against a concurrent rename.
    let current: String = sqlx::query_scalar!(
        "select handle::text as \"handle!\" from users where id = $1",
        user.id
    )
    .fetch_one(&mut *tx)
    .await?;

    if current == body.handle {
        // Idempotent no-op: same handle submitted, nothing to do.
        return Ok(StatusCode::NO_CONTENT.into_response());
    }

    // Cooldown check: a handle released by ANOTHER user within the last
    // 90 days is reserved (anti-impersonation — `released_at` is the
    // reservable-at instant, written as now()+90d below). The original
    // owner may always reclaim their own released handle (a → b → a);
    // doing so, or claiming an expired one, clears the redirect row so
    // /u/<old-handle> stops 301-ing to its previous owner.
    let redirect = sqlx::query!(
        r#"select user_id, (released_at > now()) as "in_cooldown!"
             from handle_redirects where old_handle = $1"#,
        body.handle
    )
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(r) = redirect {
        if r.user_id != user.id && r.in_cooldown {
            return Err(AppError::Conflict("handle is reserved".into()));
        }
        sqlx::query!(
            "delete from handle_redirects where old_handle = $1",
            body.handle
        )
        .execute(&mut *tx)
        .await?;
    }

    // Attempt the update; intercept the unique-constraint violation.
    let res = sqlx::query!(
        "update users set handle = $1 where id = $2",
        body.handle,
        user.id
    )
    .execute(&mut *tx)
    .await;

    if let Err(sqlx::Error::Database(ref db)) = res
        && db.constraint() == Some("users_handle_uidx")
    {
        return Err(AppError::Conflict("handle already taken".into()));
    }
    res?;

    // Insert (or refresh) the redirect row. `on conflict ... do update`
    // handles the case where a user renames a → b → a → c: the redirect
    // for `a` already exists and must point to the new user_id/released_at.
    // Released-at is now + 90 days: the old handle becomes reservable again
    // after the cooldown period. 90 days is hardcoded per spec.
    sqlx::query!(
        "insert into handle_redirects (old_handle, user_id, released_at) \
         values ($1, $2, now() + interval '90 days') \
         on conflict (old_handle) do update \
           set user_id = excluded.user_id, released_at = excluded.released_at",
        current,
        user.id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(StatusCode::OK.into_response())
}
