//! Manual target attachment to a photo. Looks up by slug or alias;
//! writes a photo_targets row with source='manual' and is_primary=true
//! when the user picked one explicitly.

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

pub async fn attach_primary_by_freetext(
    pool: &PgPool,
    photo_id: Uuid,
    freetext: &str,
) -> Result<(), AppError> {
    let trimmed = freetext.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    // Try slug exact, then alias inclusion.
    let target_id: Option<Uuid> = sqlx::query_scalar!(
        r#"
        select id from targets
         where slug = lower($1)
            or $1 = any (aliases)
            or canonical_name ilike $1
         limit 1
        "#,
        trimmed
    )
    .fetch_optional(pool)
    .await?;

    let Some(tid) = target_id else {
        return Ok(()); // unknown target, just keep photos.target
    };

    sqlx::query!(
        "insert into photo_targets (photo_id, target_id, source, is_primary) \
         values ($1, $2, 'manual', true) \
         on conflict (photo_id, target_id) do update set is_primary = true, source = 'manual'",
        photo_id,
        tid
    )
    .execute(pool)
    .await?;
    Ok(())
}
