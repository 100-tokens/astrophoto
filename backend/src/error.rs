use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("gone: {0}")]
    Gone(String),

    #[error("validation: {0}")]
    Validation(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("too many requests: {0}")]
    TooManyRequests(String),

    #[error("rate limited")]
    RateLimited,

    #[error("service unavailable")]
    ServiceUnavailable,

    #[error("quota exceeded: {0}")]
    QuotaExceeded(String),

    #[error("payload too large: {0}")]
    PayloadTooLarge(String),

    #[error("magic byte mismatch: {0}")]
    MagicByteMismatch(String),

    #[error("pending finalize stuck: {0}")]
    PendingFinalizeStuck(String),

    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error("internal: {0}")]
    Internal(String),
}

impl AppError {
    pub fn internal(msg: impl Into<String>) -> Self {
        AppError::Internal(msg.into())
    }

    pub fn bad_request(msg: impl Into<String>) -> Self {
        AppError::BadRequest(msg.into())
    }

    pub fn gone(msg: impl Into<String>) -> Self {
        AppError::Gone(msg.into())
    }

    pub fn too_many_requests(msg: impl Into<String>) -> Self {
        AppError::TooManyRequests(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        AppError::NotFound(msg.into())
    }
}

impl AppError {
    fn status(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Gone(_) => StatusCode::GONE,
            AppError::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            AppError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            AppError::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            AppError::QuotaExceeded(_) => StatusCode::PAYLOAD_TOO_LARGE,
            AppError::PayloadTooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
            AppError::MagicByteMismatch(_) => StatusCode::BAD_REQUEST,
            AppError::PendingFinalizeStuck(_) => StatusCode::REQUEST_TIMEOUT,
            AppError::UnsupportedFormat(_) => StatusCode::BAD_REQUEST,
            AppError::Database(_) | AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            AppError::NotFound(_) => "not-found",
            AppError::Unauthorized => "unauthorized",
            AppError::Forbidden => "forbidden",
            AppError::BadRequest(_) => "bad-request",
            AppError::Gone(_) => "gone",
            AppError::Validation(_) => "validation",
            AppError::Conflict(_) => "conflict",
            AppError::TooManyRequests(_) => "too-many-requests",
            AppError::RateLimited => "rate-limited",
            AppError::ServiceUnavailable => "service-unavailable",
            AppError::QuotaExceeded(_) => "quota-exceeded",
            AppError::PayloadTooLarge(_) => "payload-too-large",
            AppError::MagicByteMismatch(_) => "magic-byte-mismatch",
            AppError::PendingFinalizeStuck(_) => "pending-finalize-stuck",
            AppError::UnsupportedFormat(_) => "unsupported-format",
            AppError::Database(_) | AppError::Internal(_) => "internal",
        }
    }
}

#[derive(Serialize)]
struct Body<'a> {
    error: &'a str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        // 5xx responses must not echo the underlying error to the client —
        // sqlx/Database errors stringify to PG output that includes column,
        // constraint, and table names. Log the real error server-side and
        // ship a uniform "internal error" body to the wire.
        let message = if status.is_server_error() {
            tracing::error!(error = %self, "server error");
            "internal error".to_string()
        } else {
            self.to_string()
        };
        let body = Body {
            error: self.code(),
            message,
        };
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn not_found_maps_to_404() {
        let resp = AppError::not_found("test_resource").into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let bytes = to_bytes(resp.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["error"], "not-found");
    }

    #[tokio::test]
    async fn validation_maps_to_422() {
        let resp = AppError::Validation("bad email".into()).into_response();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let bytes = to_bytes(resp.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["error"], "validation");
        assert!(v["message"].as_str().unwrap().contains("bad email"));
    }

    #[tokio::test]
    async fn bad_request_maps_to_400() {
        let resp = AppError::bad_request("too short").into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        let bytes = to_bytes(resp.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["error"], "bad-request");
    }

    #[tokio::test]
    async fn gone_maps_to_410() {
        let resp = AppError::gone("expired_or_used").into_response();
        assert_eq!(resp.status(), StatusCode::GONE);
        let bytes = to_bytes(resp.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["error"], "gone");
    }
}
