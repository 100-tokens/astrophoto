use axum::{
    Json,
    extract::{Query, State},
};
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::AppError;
use crate::api_types::{DraftListItem, DraftListResponse};
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

#[derive(Deserialize)]
pub struct DraftsQuery {
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(q): Query<DraftsQuery>,
) -> Result<Json<DraftListResponse>, AppError> {
    let limit = q.limit.unwrap_or(24).clamp(1, 50) as i64;

    let cursor: Option<DateTime<Utc>> = match q.cursor.as_deref() {
        Some(s) => Some(
            DateTime::parse_from_rfc3339(s)
                .map_err(|_| AppError::Validation("bad cursor".into()))?
                .with_timezone(&Utc),
        ),
        None => None,
    };

    let rows = sqlx::query!(
        r#"
        select id, short_id, original_name, target, status, created_at
          from photos
         where owner_id = $1
           and published_at is null
           and ($2::timestamptz is null or created_at < $2)
         order by created_at desc
         limit $3
        "#,
        user.id,
        cursor,
        limit
    )
    .fetch_all(&state.pool)
    .await?;

    let cdn_base = state.config.cdn_base_url.trim_end_matches('/').to_string();
    let next_cursor = rows.last().map(|r| r.created_at.to_rfc3339());

    let items = rows
        .into_iter()
        .map(|r| DraftListItem {
            id: r.id.to_string(),
            short_id: r.short_id,
            original_name: r.original_name,
            target: r.target,
            status: r.status,
            created_at: r.created_at.to_rfc3339(),
            thumb_url: format!("{cdn_base}/img/{}?w=320", r.id),
        })
        .collect::<Vec<_>>();

    let next_cursor = if items.len() as i64 == limit {
        next_cursor
    } else {
        None
    };

    Ok(Json(DraftListResponse { items, next_cursor }))
}
