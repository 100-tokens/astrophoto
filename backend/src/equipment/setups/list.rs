//! GET /api/equipment/setups — caller's setups with per-role counts,
//! newest-updated first.

use axum::{Json, extract::State, response::IntoResponse};

use crate::api_types::{RoleCount, SetupSummary};
use crate::auth::middleware::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let rows = sqlx::query!(
        r#"
        select s.id, s.name, s.description, s.location,
               s.is_remote, s.is_default, s.guiding, s.updated_at,
               coalesce(
                 (select json_agg(json_build_object('role', si.role, 'count', si.cnt))
                    from (
                      select role, count(*) as cnt
                        from setup_items
                       where setup_id = s.id
                       group by role
                    ) as si),
                 '[]'::json
               ) as "item_counts!: serde_json::Value"
          from equipment_setups s
         where s.owner_id = $1
         order by s.updated_at desc
        "#,
        user.0.id
    )
    .fetch_all(&state.pool)
    .await?;

    let out: Vec<SetupSummary> = rows
        .into_iter()
        .map(|r| SetupSummary {
            id: r.id.to_string(),
            name: r.name,
            description: r.description,
            location: r.location,
            is_remote: r.is_remote,
            is_default: r.is_default,
            guiding: r.guiding,
            updated_at: r.updated_at.to_rfc3339(),
            item_counts: serde_json::from_value::<Vec<RoleCount>>(r.item_counts)
                .unwrap_or_default(),
        })
        .collect();
    Ok(Json(out))
}
