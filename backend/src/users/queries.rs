use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::AppError;

pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub password_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub async fn create_with_password(
    pool: &PgPool,
    email: &str,
    display_name: &str,
    password_hash: &str,
) -> Result<UserRow, AppError> {
    let row = sqlx::query_as!(
        UserRow,
        r#"
        insert into users (email, display_name, password_hash)
        values ($1, $2, $3)
        returning id, email::text as "email!", display_name, password_hash, created_at
        "#,
        email,
        display_name,
        password_hash,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db) if db.constraint() == Some("users_email_key") => {
            AppError::Conflict("email already in use".into())
        }
        _ => AppError::Database(e),
    })?;
    Ok(row)
}

pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<UserRow>, AppError> {
    let row = sqlx::query_as!(
        UserRow,
        r#"
        select id, email::text as "email!", display_name, password_hash, created_at
        from users where email = $1
        "#,
        email
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<UserRow>, AppError> {
    let row = sqlx::query_as!(
        UserRow,
        r#"
        select id, email::text as "email!", display_name, password_hash, created_at
        from users where id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
