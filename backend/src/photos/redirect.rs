use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect},
};
use uuid::Uuid;

use crate::error::AppError;
use crate::http::AppState;

pub async fn redirect_uuid_to_canonical(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<axum::response::Response, AppError> {
    let row = sqlx::query!(
        r#"
        select u.handle::text as "handle!", p.short_id
          from photos p join users u on u.id = p.owner_id
         where p.id = $1 and p.published_at is not null
           and u.pending_deletion_at is null
        "#,
        id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("photo".into()))?;

    let location = format!("/u/{}/p/{}", row.handle, row.short_id);
    Ok(Redirect::permanent(&location).into_response())
}
