//! RGPD / GDPR data export: `GET /api/me/export.json`.
//!
//! Returns a pretty-printed JSON attachment containing all personal data the
//! authenticated user has produced: account details, photos (with signed
//! 7-day URLs), comments, appreciations, and follow relationships.

use axum::{
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
};
use serde::Serialize;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

#[derive(Serialize)]
struct Export {
    exported_at: String,
    user: ExportUser,
    photos: Vec<ExportPhoto>,
    comments_authored: Vec<ExportComment>,
    appreciations_given: Vec<ExportAppreciation>,
    follows: ExportFollows,
}

#[derive(Serialize)]
struct ExportUser {
    id: String,
    email: String,
    display_name: String,
    created_at: String,
}

#[derive(Serialize)]
struct ExportPhoto {
    id: String,
    caption: Option<String>,
    captured_at: Option<String>,
    exif: serde_json::Value,
    original_url: String,
    thumbnail_url: Option<String>,
}

#[derive(Serialize)]
struct ExportComment {
    id: String,
    photo_id: String,
    body: String,
    created_at: String,
}

#[derive(Serialize)]
struct ExportAppreciation {
    photo_id: String,
    created_at: String,
}

#[derive(Serialize)]
struct ExportFollows {
    following: Vec<String>,
    followers: Vec<String>,
}

/// Pre-signed URL TTL: 7 days (maximum allowed by S3/R2).
const TTL_SECS: u64 = 7 * 24 * 3600;

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let u = sqlx::query!(
        r#"select email::text as "email!", display_name, created_at
             from users where id = $1"#,
        user.id
    )
    .fetch_one(&state.pool)
    .await?;

    // LEFT JOIN thumbnails using DISTINCT ON to pick one thumbnail per photo
    // (smallest size preset, or NULL when no thumbnail exists).
    let photos_rows = sqlx::query!(
        r#"select distinct on (p.id)
                  p.id,
                  p.caption,
                  p.taken_at,
                  p.exif_json,
                  p.storage_key,
                  t.storage_key as "thumbnail_key?"
             from photos p
             left join thumbnails t on t.photo_id = p.id
            where p.owner_id = $1
            order by p.id, t.size asc nulls last
            limit 10000"#,
        // Best-effort cap. Users with >10k photos contact support; an
        // unbounded SELECT could blow up memory on a giant account.
        user.id
    )
    .fetch_all(&state.pool)
    .await?;

    let mut photos = Vec::with_capacity(photos_rows.len());
    for p in photos_rows {
        let original_url = state.storage.signed_url(&p.storage_key, TTL_SECS).await?;
        let thumbnail_url = match p.thumbnail_key {
            Some(k) => Some(state.storage.signed_url(&k, TTL_SECS).await?),
            None => None,
        };
        photos.push(ExportPhoto {
            id: p.id.to_string(),
            caption: p.caption,
            captured_at: p.taken_at.map(|t| t.to_rfc3339()),
            exif: p.exif_json.unwrap_or(serde_json::Value::Null),
            original_url,
            thumbnail_url,
        });
    }

    let comments = sqlx::query!(
        "select id, photo_id, body, created_at from comments where author_id = $1",
        user.id
    )
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|r| ExportComment {
        id: r.id.to_string(),
        photo_id: r.photo_id.to_string(),
        body: r.body,
        created_at: r.created_at.to_rfc3339(),
    })
    .collect();

    let appreciations = sqlx::query!(
        "select photo_id, created_at from appreciations where user_id = $1",
        user.id
    )
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|r| ExportAppreciation {
        photo_id: r.photo_id.to_string(),
        created_at: r.created_at.to_rfc3339(),
    })
    .collect();

    let following: Vec<String> = sqlx::query_scalar!(
        r#"select followed_id::text as "f!" from follows where follower_id = $1"#,
        user.id
    )
    .fetch_all(&state.pool)
    .await?;

    let followers: Vec<String> = sqlx::query_scalar!(
        r#"select follower_id::text as "f!" from follows where followed_id = $1"#,
        user.id
    )
    .fetch_all(&state.pool)
    .await?;

    let payload = Export {
        exported_at: chrono::Utc::now().to_rfc3339(),
        user: ExportUser {
            id: user.id.to_string(),
            email: u.email,
            display_name: u.display_name,
            created_at: u.created_at.to_rfc3339(),
        },
        photos,
        comments_authored: comments,
        appreciations_given: appreciations,
        follows: ExportFollows {
            following,
            followers,
        },
    };

    let json = serde_json::to_string_pretty(&payload)
        .map_err(|e| AppError::internal(format!("export serialise: {e}")))?;
    let filename = format!(
        "astrophoto-export-{}-{}.json",
        user.id,
        chrono::Utc::now().format("%Y-%m-%d")
    );

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        json,
    ))
}
