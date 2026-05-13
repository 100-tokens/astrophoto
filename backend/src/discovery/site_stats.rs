//! GET /api/site/stats — global counts shown in the home-page hero band.
//! Cached aggressively at the CDN edge; the numbers move slowly.

use axum::{Json, extract::State};

use crate::AppError;
use crate::api_types::SiteStats;
use crate::http::AppState;

pub async fn get(State(state): State<AppState>) -> Result<Json<SiteStats>, AppError> {
    let row = sqlx::query!(
        r#"
        select
          coalesce(count(distinct owner_id), 0)::bigint as "practitioners!",
          coalesce(count(*), 0)::bigint as "frames!",
          coalesce(sum(exposure_s * coalesce(sessions, 1)), 0)::bigint as "integration_seconds!"
        from photos
        where published_at is not null
        "#
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(SiteStats {
        practitioners: row.practitioners,
        frames: row.frames,
        integration_seconds: row.integration_seconds,
    }))
}
