use axum::{Json, extract::State, response::IntoResponse};

use crate::AppError;
use crate::api_types::User;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let following_ids = sqlx::query!(
        "select followed_id from follows where follower_id = $1 limit 500",
        user.id
    )
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|r| r.followed_id.to_string())
    .collect();

    let dto = User {
        id: user.id.to_string(),
        email: user.email,
        display_name: user.display_name,
        created_at: user.created_at.to_rfc3339(),
        following_ids,
    };

    Ok(Json(dto))
}
