//! `GET /api/photos/:id/xisf-meta` — owner-only display view over the
//! persisted plate-solve response.
//!
//! Reads `platesolve_embed_json` and produces a typed
//! [`crate::photos::xisf_display::XisfDisplayMeta`] for the verify
//! form's PROCESSING HISTORY sidebar block. No DB write, no upstream
//! round-trip — this is presentation glue.

use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::Value;
use uuid::Uuid;

use crate::auth::middleware::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;
use crate::photos::xisf_display::{self, XisfDisplayMeta};

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<Json<XisfDisplayMeta>, AppError> {
    // Runtime query — `platesolve_embed_json` is jsonb; the
    // compile-time form would still work but matches the rest of the
    // platesolve module's runtime-query pattern.
    let row: Option<(Uuid, Option<Value>)> = sqlx::query_as::<_, (Uuid, Option<Value>)>(
        "select owner_id, platesolve_embed_json from photos where id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(AppError::from)?;

    let Some((owner_id, embed)) = row else {
        return Err(AppError::not_found("photo"));
    };
    if owner_id != user.id {
        // Same leak-prevention pattern as the other photo handlers:
        // hide existence under 404 rather than reveal it via 403.
        return Err(AppError::not_found("photo"));
    }

    let meta = xisf_display::extract_from_embed(embed.as_ref());
    Ok(Json(meta))
}
