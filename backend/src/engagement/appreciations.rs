//! Appreciations: idempotent ♥ toggle on a photo. Auth required to
//! mutate, public to read counts; the per-user state has its own
//! auth-required endpoint.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Serialize;
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::{CurrentUser, OptionalUser};
use crate::http::AppState;
use crate::photos::queries::is_visible_to;

#[derive(Serialize)]
pub struct CountResponse {
    pub count: i64,
}

#[derive(Serialize)]
pub struct StateResponse {
    pub appreciated: bool,
}

pub async fn appreciate(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(photo_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    if !is_visible_to(&state.pool, photo_id, Some(user.id)).await? {
        return Err(AppError::not_found("photo"));
    }
    sqlx::query!(
        r#"
        insert into appreciations (user_id, photo_id)
        values ($1, $2)
        on conflict (user_id, photo_id) do nothing
        "#,
        user.id,
        photo_id
    )
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn unappreciate(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(photo_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    if !is_visible_to(&state.pool, photo_id, Some(user.id)).await? {
        return Err(AppError::not_found("photo"));
    }
    sqlx::query!(
        "delete from appreciations where user_id = $1 and photo_id = $2",
        user.id,
        photo_id
    )
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn count(
    State(state): State<AppState>,
    user: OptionalUser,
    Path(photo_id): Path<Uuid>,
) -> Result<Json<CountResponse>, AppError> {
    if !is_visible_to(&state.pool, photo_id, user.0.as_ref().map(|u| u.id)).await? {
        return Err(AppError::not_found("photo"));
    }
    let row = sqlx::query!(
        r#"select count(*) as "count!" from appreciations where photo_id = $1"#,
        photo_id
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(CountResponse { count: row.count }))
}

pub async fn state_for_user(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(photo_id): Path<Uuid>,
) -> Result<Json<StateResponse>, AppError> {
    if !is_visible_to(&state.pool, photo_id, Some(user.id)).await? {
        return Err(AppError::not_found("photo"));
    }
    let row = sqlx::query!(
        "select 1 as one from appreciations where user_id = $1 and photo_id = $2 limit 1",
        user.id,
        photo_id
    )
    .fetch_optional(&state.pool)
    .await?;
    Ok(Json(StateResponse {
        appreciated: row.is_some(),
    }))
}
