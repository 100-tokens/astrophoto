use axum::{Json, extract::State, response::IntoResponse};

use crate::AppError;
use crate::api_types::User;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let row = sqlx::query!(
        r#"
        select u.pending_deletion_at,
               coalesce(array_agg(f.followed_id) filter (where f.followed_id is not null), '{}') as "ids!: Vec<uuid::Uuid>"
          from users u
          left join follows f on f.follower_id = u.id
         where u.id = $1
         group by u.id
        "#,
        user.id
    )
    .fetch_one(&state.pool)
    .await?;

    let following_ids: Vec<String> = row.ids.iter().take(500).map(|id| id.to_string()).collect();

    let dto = User {
        id: user.id.to_string(),
        email: user.email,
        display_name: user.display_name,
        created_at: user.created_at.to_rfc3339(),
        following_ids,
        pending_deletion_at: row
            .pending_deletion_at
            .map(|t: chrono::DateTime<chrono::Utc>| t.to_rfc3339()),
    };

    Ok(Json(dto))
}
