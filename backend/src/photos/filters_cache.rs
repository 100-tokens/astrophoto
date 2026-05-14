//! Rebuild `photos.filters` (the legacy comma-joined display-names cache)
//! from the `photo_filters` junction. Called by every writer of the
//! junction in the same transaction.
//!
//! Junction is the source of truth; cache exists for the older browse
//! indexes that still scan `photos.filters` directly.

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::error::AppError;

/// Recompute the cache string for `photo_id`. Empty junction → NULL cache.
pub async fn rebuild(tx: &mut Transaction<'_, Postgres>, photo_id: Uuid) -> Result<(), AppError> {
    let rows = sqlx::query!(
        r#"select e.display_name as "display_name!"
             from photo_filters pf
             join equipment_items e on e.id = pf.item_id
            where pf.photo_id = $1
            order by pf.position, e.display_name"#,
        photo_id
    )
    .fetch_all(&mut **tx)
    .await?;

    let joined: Option<String> = if rows.is_empty() {
        None
    } else {
        Some(
            rows.iter()
                .map(|r| r.display_name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        )
    };

    sqlx::query!(
        "update photos set filters = $1 where id = $2",
        joined,
        photo_id
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}
