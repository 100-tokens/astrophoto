//! `GET/PUT /api/admin/settings` — read and update runtime app settings.

use axum::{Json, extract::State, response::IntoResponse};

use crate::AppError;
use crate::auth::middleware::AdminUser;
use crate::http::AppState;
use crate::settings::{self, AppSettings};

/// Current settings. (The reader is fail-safe; this always returns a value.)
pub async fn get(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(settings::get(&state.pool).await))
}

/// Replace all settings. Validates bounds, stamps the editing admin, returns
/// the persisted values.
pub async fn put(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Json(body): Json<AppSettings>,
) -> Result<impl IntoResponse, AppError> {
    let updated = settings::update(&state.pool, &body, admin.id).await?;
    Ok(Json(updated))
}
