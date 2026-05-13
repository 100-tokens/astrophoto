//! Follows: asymmetric, idempotent toggle. Auth required to mutate.
//! Counts public.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Serialize;
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

#[derive(Serialize)]
pub struct CountResponse {
    pub count: i64,
}

pub async fn follow(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(target_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    if user.id == target_id {
        return Err(AppError::Validation("cannot follow yourself".into()));
    }
    let res = sqlx::query!(
        r#"
        insert into follows (follower_id, followed_id)
        values ($1, $2)
        on conflict (follower_id, followed_id) do nothing
        "#,
        user.id,
        target_id,
    )
    .execute(&state.pool)
    .await;
    match res {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        // Postgres 23503 = foreign_key_violation. The only FK on this insert
        // that can fail with a syntactically valid UUID is followed_id → users,
        // so a 23503 here means the target user does not exist. Map to 404
        // rather than leaking the constraint name in a 500.
        Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23503") => {
            Err(AppError::not_found("user"))
        }
        Err(e) => Err(AppError::Database(e)),
    }
}

pub async fn unfollow(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(target_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    sqlx::query!(
        "delete from follows where follower_id = $1 and followed_id = $2",
        user.id,
        target_id,
    )
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn followers_count(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<CountResponse>, AppError> {
    let row = sqlx::query!(
        r#"select count(*) as "count!" from follows where followed_id = $1"#,
        user_id
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(CountResponse { count: row.count }))
}

pub async fn following_count(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<CountResponse>, AppError> {
    let row = sqlx::query!(
        r#"select count(*) as "count!" from follows where follower_id = $1"#,
        user_id
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(CountResponse { count: row.count }))
}
