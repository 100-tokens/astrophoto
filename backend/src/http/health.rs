use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;
use sqlx::PgPool;

use crate::AppError;

#[derive(Serialize)]
pub struct HealthBody {
    pub status: &'static str,
    pub db: &'static str,
}

pub async fn healthz(
    State(pool): State<PgPool>,
) -> Result<(StatusCode, Json<HealthBody>), AppError> {
    sqlx::query_scalar::<_, i32>("select 1")
        .fetch_one(&pool)
        .await?;
    Ok((
        StatusCode::OK,
        Json(HealthBody {
            status: "ok",
            db: "ok",
        }),
    ))
}
