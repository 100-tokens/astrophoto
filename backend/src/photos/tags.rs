//! Tag write helpers: slug-normalize, upsert into `tags`, attach to a
//! photo via `photo_tags`. Cap is enforced upstream (max 8) by the
//! caller (the verify endpoint).

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

pub fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

pub async fn attach(
    pool: &PgPool,
    photo_id: Uuid,
    tags_freetext: &[String],
) -> Result<(), AppError> {
    if tags_freetext.is_empty() {
        return Ok(());
    }
    if tags_freetext.len() > 8 {
        return Err(AppError::Validation("max 8 tags".into()));
    }

    for t in tags_freetext {
        let slug = slugify(t);
        if slug.is_empty() {
            continue;
        }
        let tag_id: Uuid = sqlx::query_scalar!(
            r#"
            insert into tags (slug, name) values ($1, $2)
            on conflict (slug) do update set slug = excluded.slug
            returning id
            "#,
            slug,
            t.trim()
        )
        .fetch_one(pool)
        .await?;

        sqlx::query!(
            "insert into photo_tags (photo_id, tag_id) values ($1, $2) \
             on conflict do nothing",
            photo_id,
            tag_id
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_lowercases_and_dashes() {
        assert_eq!(slugify("Wide Field"), "wide-field");
        assert_eq!(slugify("H-Alpha"), "h-alpha");
        assert_eq!(slugify("NGC 7000"), "ngc-7000");
        assert_eq!(slugify("  trimmed  "), "trimmed");
        assert_eq!(slugify("---bad-edges---"), "bad-edges");
    }
}
