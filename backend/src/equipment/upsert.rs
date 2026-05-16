//! equipment_items upsert. Called whenever a photo's equipment fields
//! are written. Increments usage_count on existing entries; otherwise
//! creates a new row in title-case display form with lowercase canonical.

use sqlx::PgPool;

use crate::error::AppError;

pub async fn upsert(pool: &PgPool, kind: &str, freetext: &str) -> Result<(), AppError> {
    let display = freetext.trim();
    if display.is_empty() {
        return Ok(());
    }
    let canonical = display.to_lowercase();
    sqlx::query!(
        r#"
        insert into equipment_items
            (kind, canonical_name, display_name, usage_count, approved_at)
            values ($1, $2, $3, 1, now())
        on conflict (kind, canonical_name)
            do update set usage_count = equipment_items.usage_count + 1
        "#,
        kind,
        canonical,
        display
    )
    .execute(pool)
    .await?;
    Ok(())
}
