use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::AppError;

pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub handle: String,
    pub display_name: String,
    pub password_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub is_admin: bool,
}

pub async fn create_with_password(
    pool: &PgPool,
    email: &str,
    handle: &str,
    display_name: &str,
    password_hash: &str,
) -> Result<UserRow, AppError> {
    sqlx::query_as!(
        UserRow,
        r#"
        insert into users (email, handle, display_name, password_hash, password_changed_at)
        values ($1, $2, $3, $4, now())
        returning id, email::text as "email!", handle::text as "handle!", display_name, password_hash, created_at, email_verified_at, is_admin
        "#,
        email,
        handle,
        display_name,
        password_hash,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db) if db.constraint() == Some("users_email_key") => {
            AppError::Conflict("email already in use".into())
        }
        sqlx::Error::Database(db) if db.constraint() == Some("users_handle_uidx") => {
            AppError::Conflict("handle already taken".into())
        }
        _ => AppError::Database(e),
    })
}

pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<UserRow>, AppError> {
    let row = sqlx::query_as!(
        UserRow,
        r#"
        select id, email::text as "email!", handle::text as "handle!", display_name, password_hash, created_at, email_verified_at, is_admin
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
        select id, email::text as "email!", handle::text as "handle!", display_name, password_hash, created_at, email_verified_at, is_admin
        from users where id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

#[cfg(test)]
mod tests_email_verified {
    use super::*;

    #[tokio::test]
    async fn new_password_account_has_no_email_verified_at() {
        let pg = testcontainers::runners::AsyncRunner::start(testcontainers::ImageExt::with_tag(
            testcontainers_modules::postgres::Postgres::default(),
            "16-alpine",
        ))
        .await
        .unwrap();
        let host = pg.get_host().await.unwrap();
        let port = pg.get_host_port_ipv4(5432).await.unwrap();
        let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let user = create_with_password(&pool, "u@example.com", "u-abc", "U", "hash")
            .await
            .unwrap();
        assert!(user.email_verified_at.is_none());
    }

    #[tokio::test]
    async fn find_by_email_returns_verified_timestamp() {
        let pg = testcontainers::runners::AsyncRunner::start(testcontainers::ImageExt::with_tag(
            testcontainers_modules::postgres::Postgres::default(),
            "16-alpine",
        ))
        .await
        .unwrap();
        let host = pg.get_host().await.unwrap();
        let port = pg.get_host_port_ipv4(5432).await.unwrap();
        let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let user = create_with_password(&pool, "v@example.com", "v-abc", "V", "hash")
            .await
            .unwrap();
        sqlx::query!(
            "update users set email_verified_at = now() where id = $1",
            user.id
        )
        .execute(&pool)
        .await
        .unwrap();
        let fetched = find_by_email(&pool, "v@example.com")
            .await
            .unwrap()
            .unwrap();
        assert!(fetched.email_verified_at.is_some());
    }
}
