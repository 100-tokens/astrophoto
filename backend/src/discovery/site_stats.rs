//! GET /api/site/stats — global counts shown in the home-page hero band.
//!
//! The aggregate scans every published photo, and both the home and the
//! explore pages fetch this on every SSR render — so the result is
//! memoized in-process for [`TTL`] and served with a Cache-Control
//! header. No CDN fronts the API in the current Koyeb topology; the
//! header only helps if one ever does.

use std::time::{Duration, Instant};

use axum::{Json, extract::State, http::header, response::IntoResponse};

use crate::AppError;
use crate::api_types::SiteStats;
use crate::http::AppState;

/// TTL for the in-process memo. The numbers move slowly (an upload
/// changes one count by one), so up-to-a-minute staleness is invisible.
const TTL: Duration = Duration::from_secs(60);

/// `(computed_at, practitioners, frames, integration_seconds)`.
static CACHE: tokio::sync::RwLock<Option<(Instant, i64, i64, i64)>> =
    tokio::sync::RwLock::const_new(None);

pub async fn get(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let cached = *CACHE.read().await;
    let (practitioners, frames, integration_seconds) = match cached {
        Some((at, p, f, s)) if at.elapsed() < TTL => (p, f, s),
        _ => {
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
            *CACHE.write().await = Some((
                Instant::now(),
                row.practitioners,
                row.frames,
                row.integration_seconds,
            ));
            (row.practitioners, row.frames, row.integration_seconds)
        }
    };

    Ok((
        [(header::CACHE_CONTROL, "public, max-age=60")],
        Json(SiteStats {
            practitioners,
            frames,
            integration_seconds,
        }),
    ))
}
