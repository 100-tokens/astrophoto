use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};

use crate::AppError;
use crate::api_types::{
    EquipmentSummary, LocationSummary, Profile, ProfilePatch, SocialLink,
};
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;
use crate::users::{bio, social_links};

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

const MAX_BIO_HTML_BYTES: usize = 16_384;
const MAX_TAGLINE_CHARS:  usize = 140;
const MAX_DISPLAY_NAME_CHARS: usize = 60;

pub async fn put(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<ProfilePatch>,
) -> Result<impl IntoResponse, AppError> {
    // ── 1. Validate everything before any DB write ─────────────────────
    if let Some(Some(name)) = body.display_name.as_ref() {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.chars().count() > MAX_DISPLAY_NAME_CHARS {
            return Err(AppError::bad_request("invalid_display_name"));
        }
    }
    if let Some(Some(tag)) = body.tagline.as_ref() {
        if tag.chars().count() > MAX_TAGLINE_CHARS {
            return Err(AppError::bad_request("tagline_too_long"));
        }
    }

    // bio_html: sanitise BEFORE any other check; never trust the client.
    let bio_sanitised: Option<Option<String>> = body.bio_html.as_ref().map(|outer| {
        outer.as_ref().map(|raw| {
            let cleaned = bio::sanitize(raw);
            // Bytes, not chars — ammonia returns UTF-8 bytes; the column is text but
            // we cap on serialised length to bound payloads.
            if cleaned.len() > MAX_BIO_HTML_BYTES {
                cleaned.chars().take(MAX_BIO_HTML_BYTES).collect()
            } else {
                cleaned
            }
        })
    });

    if let Some(loc) = body.location.as_ref() {
        if let Some(b) = loc.bortle_class {
            if !(1..=9).contains(&b) {
                return Err(AppError::bad_request("bortle_out_of_range"));
            }
        }
        if let Some(s) = loc.sqm {
            if !(0.0..=99.99).contains(&s) {
                return Err(AppError::bad_request("sqm_out_of_range"));
            }
        }
    }

    if let Some(links) = body.social_links.as_ref() {
        social_links::validate_links(links)?;
    }

    // ── 2. Transactional per-column conditional UPDATE ─────────────────
    let mut tx = state.pool.begin().await?;

    if let Some(opt) = body.display_name {
        // display_name is NOT NULL — explicit null is a no-op (no UPDATE issued).
        if let Some(name) = opt.as_ref() {
            let trimmed = name.trim();
            sqlx::query!(
                "update users set display_name = $1 where id = $2",
                trimmed,
                user.id
            )
            .execute(&mut *tx)
            .await?;
        }
    }
    if let Some(opt) = body.tagline {
        sqlx::query!(
            "update users set tagline = $1 where id = $2",
            opt.as_deref(),
            user.id
        )
        .execute(&mut *tx)
        .await?;
    }
    if let Some(opt) = bio_sanitised {
        sqlx::query!(
            "update users set bio_html = $1 where id = $2",
            opt.as_deref(),
            user.id
        )
        .execute(&mut *tx)
        .await?;
    }
    if let Some(eq) = body.equipment {
        sqlx::query!(
            r#"
            update users set
                equipment_telescope = $1,
                equipment_camera    = $2,
                equipment_mount     = $3,
                equipment_filters   = $4,
                equipment_guiding   = $5
            where id = $6
            "#,
            eq.telescope.as_deref(),
            eq.camera.as_deref(),
            eq.mount.as_deref(),
            eq.filters.as_deref(),
            eq.guiding.as_deref(),
            user.id
        )
        .execute(&mut *tx)
        .await?;
    }
    if let Some(loc) = body.location {
        // sqm is `numeric(4,2)`; cast a Postgres double to numeric server-side
        // so we don't need a Decimal crate on the Rust side.
        sqlx::query!(
            r#"
            update users set
                location_text = $1,
                bortle_class  = $2,
                sqm           = cast($3::float8 as numeric(4,2))
            where id = $4
            "#,
            loc.location_text.as_deref(),
            loc.bortle_class,
            loc.sqm,
            user.id
        )
        .execute(&mut *tx)
        .await?;
    }
    if let Some(links) = body.social_links {
        let json = serde_json::to_value(&links)
            .map_err(|_| AppError::internal("social_links_serialise"))?;
        sqlx::query!(
            "update users set social_links = $1 where id = $2",
            json,
            user.id
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
