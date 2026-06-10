use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

use crate::auth::handle::{HandleError, validate};
use crate::error::AppError;
use crate::http::AppState;

#[derive(Deserialize)]
pub struct Q {
    pub handle: String,
}

#[derive(Serialize)]
pub struct R {
    pub status: &'static str,
}

pub async fn handler(
    State(state): State<AppState>,
    Query(q): Query<Q>,
) -> Result<impl IntoResponse, AppError> {
    match validate(&q.handle) {
        Err(HandleError::Format) => return Ok(Json(R { status: "invalid" })),
        Err(HandleError::Reserved) => return Ok(Json(R { status: "reserved" })),
        Ok(()) => {}
    }

    // A handle is unavailable when a user holds it OR when it was renamed
    // away less than 90 days ago: `handle_redirects.released_at` stores the
    // instant the handle becomes reservable again (now()+90d at rename
    // time — the anti-impersonation cooldown from migration 0005).
    let taken: bool = sqlx::query_scalar!(
        r#"select exists(select 1 from users where handle = $1)
               or exists(select 1 from handle_redirects
                          where old_handle = $1 and released_at > now())"#,
        q.handle
    )
    .fetch_one(&state.pool)
    .await?
    .unwrap_or(false);

    Ok(Json(R {
        status: if taken { "taken" } else { "available" },
    }))
}
