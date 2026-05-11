//! GET /api/me/storage — surfaces per-owner storage usage and quota
//! so the upload page can render "STORAGE · 1.84 / 5.00 GB USED".

use axum::{Json, extract::State};

use crate::AppError;
use crate::api_types::{StorageSummary, UserTier};
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

// Quota ceilings — match the design's "5 GB free / 50 GB subscriber"
// reference. The values live next to the response shape so anyone
// adjusting tier limits flips both quotas and the documented copy.
const FREE_QUOTA_BYTES: i64 = 5 * 1024 * 1024 * 1024;
const SUBSCRIBER_QUOTA_BYTES: i64 = 50 * 1024 * 1024 * 1024;

pub async fn summary(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<StorageSummary>, AppError> {
    // Two queries beats a single LEFT JOIN + GROUP BY here — photos has
    // no soft-delete column, so the join would just sum bytes for all
    // owned rows. Doing it as two scalar queries keeps the SQL trivial
    // and the planner happy.
    let used: i64 = sqlx::query_scalar!(
        r#"select coalesce(sum(bytes), 0)::bigint as "used!: i64"
             from photos where owner_id = $1"#,
        user.id
    )
    .fetch_one(&state.pool)
    .await?;
    let tier_str: String = sqlx::query_scalar!(r#"select tier from users where id = $1"#, user.id)
        .fetch_one(&state.pool)
        .await?;

    let tier = match tier_str.as_str() {
        "subscriber" => UserTier::Subscriber,
        _ => UserTier::Free,
    };
    let quota_bytes = match tier {
        UserTier::Subscriber => SUBSCRIBER_QUOTA_BYTES,
        UserTier::Free => FREE_QUOTA_BYTES,
    };

    Ok(Json(StorageSummary {
        used_bytes: used,
        quota_bytes,
        tier,
    }))
}
