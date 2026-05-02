//! Comments: 1-level (flat). Anyone can read, auth required to post.
//! Delete authorized for the comment author OR the photo owner.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::AppError;
use crate::auth::middleware::{CurrentUser, OptionalUser};
use crate::http::AppState;
use crate::photos::queries::is_visible_to;

#[derive(Serialize)]
pub struct Comment {
    pub id: String,
    pub photo_id: String,
    pub author_id: Option<String>,
    pub author_display_name: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct ListResponse {
    pub comments: Vec<Comment>,
}

#[derive(Deserialize, Validate)]
pub struct CreateBody {
    #[validate(length(min = 1, max = 2000))]
    pub body: String,
}

struct CommentRow {
    id: Uuid,
    photo_id: Uuid,
    // Nullable since migration 0003: pseudonymised comments have author_id = NULL.
    author_id: Option<Uuid>,
    author_display_name: String,
    body: String,
    created_at: DateTime<Utc>,
}

impl From<CommentRow> for Comment {
    fn from(r: CommentRow) -> Self {
        Comment {
            id: r.id.to_string(),
            photo_id: r.photo_id.to_string(),
            // Deleted-account comments surface with author_id = null and display_name = "[deleted]".
            author_id: r.author_id.map(|u| u.to_string()),
            author_display_name: r.author_display_name,
            body: r.body,
            created_at: r.created_at.to_rfc3339(),
        }
    }
}

pub async fn list(
    State(state): State<AppState>,
    user: OptionalUser,
    Path(photo_id): Path<Uuid>,
) -> Result<Json<ListResponse>, AppError> {
    if !is_visible_to(&state.pool, photo_id, user.0.as_ref().map(|u| u.id)).await? {
        return Err(AppError::not_found("photo"));
    }
    let rows = sqlx::query_as!(
        CommentRow,
        r#"
        select c.id, c.photo_id, c.author_id,
               coalesce(u.display_name, '[deleted]') as "author_display_name!: String",
               c.body, c.created_at
        from comments c
        left join users u on u.id = c.author_id
        where c.photo_id = $1
        order by c.created_at asc
        "#,
        photo_id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(ListResponse {
        comments: rows.into_iter().map(Into::into).collect(),
    }))
}

pub async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(photo_id): Path<Uuid>,
    Json(body): Json<CreateBody>,
) -> Result<(StatusCode, Json<Comment>), AppError> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    if !is_visible_to(&state.pool, photo_id, Some(user.id)).await? {
        return Err(AppError::not_found("photo"));
    }

    // Verify the photo exists; surfaces 404 (photos table FK enforces it
    // anyway, but better error message than a constraint violation).
    let exists = sqlx::query!("select id from photos where id = $1", photo_id)
        .fetch_optional(&state.pool)
        .await?;
    if exists.is_none() {
        return Err(AppError::not_found("photo"));
    }

    let row = sqlx::query_as!(
        CommentRow,
        r#"
        with inserted as (
            insert into comments (photo_id, author_id, body)
            values ($1, $2, $3)
            returning id, photo_id, author_id, body, created_at
        )
        select i.id, i.photo_id, i.author_id,
               u.display_name as author_display_name,
               i.body, i.created_at
        from inserted i
        join users u on u.id = i.author_id
        "#,
        photo_id,
        user.id,
        body.body,
    )
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(row.into())))
}

pub async fn delete(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(comment_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    // Look up: who wrote it AND who owns the photo it's on.
    let row = sqlx::query!(
        r#"
        select c.author_id, p.owner_id as photo_owner_id
        from comments c
        join photos p on p.id = c.photo_id
        where c.id = $1
        "#,
        comment_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::not_found("comment"))?;

    // author_id is nullable (NULL when account was deleted). A deleted-account
    // comment can only be removed by the photo owner.
    if row.author_id != Some(user.id) && row.photo_owner_id != user.id {
        return Err(AppError::Forbidden);
    }

    sqlx::query!("delete from comments where id = $1", comment_id)
        .execute(&state.pool)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
