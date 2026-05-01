use axum::{
    extract::State,
    http::{HeaderMap, StatusCode, header::COOKIE},
    response::IntoResponse,
};

use crate::auth::session;
use crate::http::AppState;
use crate::AppError;

#[allow(clippy::unwrap_used)]
pub async fn handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    if let Some(value) = headers
        .get(COOKIE)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| {
            s.split(';')
                .map(str::trim)
                .find_map(|kv| kv.strip_prefix(&format!("{}=", session::COOKIE_NAME)))
        })
    {
        session::delete(&state.pool, value).await?;
    }
    let clear = session::clear_cookie_header(state.config.session_secure);
    let mut resp = StatusCode::NO_CONTENT.into_response();
    resp.headers_mut()
        .insert("set-cookie", clear.parse().unwrap());
    Ok(resp)
}
