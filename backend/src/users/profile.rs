use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

use crate::AppError;
use crate::api_types::{EquipmentSummary, LocationSummary, Profile, SocialLink};
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

pub async fn get(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Profile>, AppError> {
    let row = sqlx::query!(
        r#"
        select
            display_name,
            tagline,
            bio_html,
            cover_photo_id,
            equipment_telescope,
            equipment_camera,
            equipment_mount,
            equipment_filters,
            equipment_guiding,
            location_text,
            bortle_class,
            cast(sqm as double precision) as "sqm?: f64",
            social_links
        from users
        where id = $1
        "#,
        user.id
    )
    .fetch_one(&state.pool)
    .await?;

    let social_links: Vec<SocialLink> = serde_json::from_value(row.social_links)
        .map_err(|_| AppError::internal("social_links_corrupt"))?;

    Ok(Json(Profile {
        display_name: row.display_name,
        tagline: row.tagline,
        bio_html: row.bio_html,
        cover_photo_id: row.cover_photo_id,
        equipment: EquipmentSummary {
            telescope: row.equipment_telescope,
            camera: row.equipment_camera,
            mount: row.equipment_mount,
            filters: row.equipment_filters,
            guiding: row.equipment_guiding,
        },
        location: LocationSummary {
            location_text: row.location_text,
            bortle_class: row.bortle_class,
            sqm: row.sqm,
        },
        social_links,
    }))
}

#[derive(Deserialize)]
pub struct PutBody {
    pub display_name: Option<String>,
}

pub async fn put(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<PutBody>,
) -> Result<impl IntoResponse, AppError> {
    if let Some(name) = body.display_name {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.chars().count() > 60 {
            return Err(AppError::bad_request("invalid_display_name"));
        }
        sqlx::query!(
            "update users set display_name = $1 where id = $2",
            trimmed,
            user.id
        )
        .execute(&state.pool)
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}
