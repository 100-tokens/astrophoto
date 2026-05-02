use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::AppError;
use crate::api_types::SessionRow;
use crate::auth::middleware::{CurrentSessionId, CurrentUser};
use crate::http::AppState;

fn parse_label(ua: &str) -> (String, String, String, String, String) {
    let parser = woothee::parser::Parser::new();
    match parser.parse(ua) {
        Some(r) => (
            r.name.to_string(),
            r.version.to_string(),
            r.os.to_string(),
            r.os_version.to_string(),
            r.category.to_string(),
        ),
        None => (
            "unknown".into(),
            String::new(),
            "unknown".into(),
            String::new(),
            "unknown".into(),
        ),
    }
}

pub async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    CurrentSessionId(current_id): CurrentSessionId,
) -> Result<Json<Vec<SessionRow>>, AppError> {
    let rows = sqlx::query!(
        r#"select id, user_agent, ip,
                  last_used_at, created_at,
                  (id = $2) as "is_current!: bool"
             from sessions
            where user_id = $1 and expires_at > now()
            order by (id = $2) desc, last_used_at desc"#,
        user.id,
        current_id
    )
    .fetch_all(&state.pool)
    .await?;

    let out = rows
        .into_iter()
        .map(|r| {
            let ua = r.user_agent.unwrap_or_default();
            let (browser, bv, os, osv, cat) = parse_label(&ua);
            SessionRow {
                id: hex::encode(&r.id),
                browser,
                browser_version: bv,
                os,
                os_version: osv,
                category: cat,
                ip: r.ip.map(|n| n.to_string()).unwrap_or_default(),
                last_used_at: r.last_used_at.to_rfc3339(),
                created_at: r.created_at.to_rfc3339(),
                is_current: r.is_current,
            }
        })
        .collect();
    Ok(Json(out))
}

pub async fn revoke(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    CurrentSessionId(current_id): CurrentSessionId,
    Path(id_hex): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let id = hex::decode(&id_hex).map_err(|_| AppError::bad_request("bad_id"))?;
    if id == current_id {
        return Err(AppError::bad_request("use_logout"));
    }
    let res = sqlx::query!(
        "delete from sessions where id = $1 and user_id = $2",
        id,
        user.id
    )
    .execute(&state.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::not_found("session"));
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn sign_out_others(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    CurrentSessionId(current_id): CurrentSessionId,
) -> Result<impl IntoResponse, AppError> {
    sqlx::query!(
        "delete from sessions where user_id = $1 and id != $2",
        user.id,
        current_id
    )
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
