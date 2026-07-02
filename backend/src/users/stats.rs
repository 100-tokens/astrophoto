use axum::{Json, extract::State};
use serde::Serialize;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

#[derive(Serialize)]
pub struct MeStats {
    pub published_count: i64,
    pub draft_count: i64,
    pub integration_secs: f64,
    pub appreciations_received: i64,
}

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<MeStats>, AppError> {
    let row = sqlx::query!(
        r#"
        select
          count(*) filter (where published_at is not null) as "pub_count!",
          count(*) filter (where published_at is null)     as "draft_count!",
          coalesce(sum(coalesce(integration_s, exposure_s * coalesce(sessions, 1)))
            filter (where published_at is not null), 0)::float8
            as "integ!"
        from photos
        where owner_id = $1
        "#,
        user.id
    )
    .fetch_one(&state.pool)
    .await?;

    let appreciations_received = sqlx::query_scalar!(
        r#"
        select count(*) as "c!"
        from appreciations a
        join photos p on p.id = a.photo_id
        where p.owner_id = $1 and p.published_at is not null
        "#,
        user.id
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(MeStats {
        published_count: row.pub_count,
        draft_count: row.draft_count,
        integration_secs: row.integ,
        appreciations_received,
    }))
}
