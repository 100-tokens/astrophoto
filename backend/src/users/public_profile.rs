use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use chrono::Datelike;

use crate::AppError;
use crate::api_types::{
    EquipmentSummary, FeaturedPhotoSummary, HeroStats, LocationSummary, PublicProfile, SocialLink,
};
use crate::http::AppState;

pub async fn get(
    State(state): State<AppState>,
    Path(handle): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let user = sqlx::query!(
        r#"
        select id, display_name, handle, tagline, bio_html,
               cover_photo_id, avatar_id,
               equipment_telescope, equipment_camera, equipment_mount,
               equipment_filters,   equipment_guiding,
               location_text, bortle_class,
               cast(sqm as double precision) as "sqm?: f64",
               social_links,
               created_at
        from users
        where handle = $1
        "#,
        handle.to_lowercase()
    )
    .fetch_optional(&state.pool)
    .await?;

    let Some(u) = user else {
        return Err(AppError::not_found("user"));
    };

    // Featured photos (ordered by position).
    let featured_rows = sqlx::query!(
        r#"
        select p.id, p.short_id, p.featured_position, p.target,
               p.appreciations_count, p.blurhash, p.width, p.height
        from photos p
        where p.owner_id = $1 and p.featured_at is not null
        order by p.featured_position
        "#,
        u.id
    )
    .fetch_all(&state.pool)
    .await?;

    let featured: Vec<FeaturedPhotoSummary> = featured_rows
        .into_iter()
        .map(|r| FeaturedPhotoSummary {
            id: r.id,
            short_id: r.short_id,
            featured_position: r.featured_position.unwrap_or(0),
            target: r.target,
            appreciations_count: r.appreciations_count,
            blurhash: r.blurhash,
            width: r.width,
            height: r.height,
        })
        .collect();

    let cover = if let Some(cov_id) = u.cover_photo_id {
        sqlx::query!(
            r#"
            select id, short_id, target, appreciations_count, blurhash, width, height
            from photos
            where id = $1
            "#,
            cov_id
        )
        .fetch_optional(&state.pool)
        .await?
        .map(|r| FeaturedPhotoSummary {
            id: r.id,
            short_id: r.short_id,
            featured_position: 0,
            target: r.target,
            appreciations_count: r.appreciations_count,
            blurhash: r.blurhash,
            width: r.width,
            height: r.height,
        })
    } else {
        None
    };

    // Stats — single round-trip across photos + photo_targets.
    let stats_row = sqlx::query!(
        r#"
        select
            count(distinct p.id) filter (where p.published_at is not null and p.status='ready')                  as frames,
            coalesce(floor(sum(p.exposure_s) filter (where p.published_at is not null)), 0)::int8                 as integration_seconds,
            coalesce(sum(p.appreciations_count) filter (where p.published_at is not null), 0)::int8               as appreciations,
            count(distinct pt.target_id) filter (where p.published_at is not null)                                as targets
        from photos p
        left join photo_targets pt on pt.photo_id = p.id
        where p.owner_id = $1
        "#,
        u.id
    )
    .fetch_one(&state.pool)
    .await?;

    let followers: i64 =
        sqlx::query_scalar!("select count(*) from follows where followed_id = $1", u.id)
            .fetch_one(&state.pool)
            .await?
            .unwrap_or(0);

    let social_links: Vec<SocialLink> = serde_json::from_value(u.social_links)
        .map_err(|_| AppError::internal("social_links_corrupt"))?;

    Ok(Json(PublicProfile {
        id: u.id,
        handle: u.handle,
        display_name: u.display_name,
        tagline: u.tagline,
        bio_html: u.bio_html,
        cover,
        avatar_id: u.avatar_id,
        equipment: EquipmentSummary {
            telescope: u.equipment_telescope,
            camera: u.equipment_camera,
            mount: u.equipment_mount,
            filters: u.equipment_filters,
            guiding: u.equipment_guiding,
        },
        location: LocationSummary {
            location_text: u.location_text,
            bortle_class: u.bortle_class,
            sqm: u.sqm,
        },
        social_links,
        stats: HeroStats {
            frames: stats_row.frames.unwrap_or(0),
            integration_seconds: stats_row.integration_seconds.unwrap_or(0),
            followers,
            appreciations: stats_row.appreciations.unwrap_or(0),
            targets: stats_row.targets.unwrap_or(0),
            member_since_year: u.created_at.year(),
        },
        featured,
    }))
}
