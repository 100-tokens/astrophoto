//! Session expiry [audit TEST-6].
//!
//! 1. An expired session cookie must be rejected (the lookup filters on
//!    `expires_at > now()`, see `auth/session.rs::lookup`).
//! 2. `jobs::purge_deletions::purge_expired_auth_rows` deletes sessions
//!    7+ days past expiry and month-old one-shot auth tokens, leaving
//!    fresh rows alone.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use astrophoto::jobs::purge_deletions::purge_expired_auth_rows;
use common::TestApp;

#[tokio::test]
async fn expired_session_is_rejected_with_401() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app
        .signup_with_handle("Expiry User", "expiring", "expiry@example.com")
        .await;

    // Sanity: the fresh session authenticates.
    let (status, _) = app
        .oneshot("GET", "/api/auth/me", Some(&cookie), None)
        .await;
    assert_eq!(status, 200);

    // Force-expire every session of this user.
    sqlx::query!(
        "update sessions set expires_at = now() - interval '1 day' where user_id = $1",
        uid
    )
    .execute(&app.pool)
    .await
    .unwrap();

    let (status, _) = app
        .oneshot("GET", "/api/auth/me", Some(&cookie), None)
        .await;
    assert_eq!(status, 401, "expired session must no longer authenticate");
}

#[tokio::test]
async fn purge_deletes_sessions_seven_days_past_expiry_keeps_rest() {
    let app = TestApp::launch().await;
    let (_cookie, uid) = app
        .signup_with_handle("Purge User", "purgeme", "purge@example.com")
        .await;

    // Login created one active session. Add three more by hand:
    //   a) expired 8 days ago  -> purged
    //   b) expired 6 days ago  -> kept (inside the 7-day retention window)
    //   c) active for 29 days  -> kept
    for (id_byte, interval) in [
        (1u8, "now() - interval '8 days'"),
        (2u8, "now() - interval '6 days'"),
        (3u8, "now() + interval '29 days'"),
    ] {
        let sql =
            format!("insert into sessions (id, user_id, expires_at) values ($1, $2, {interval})");
        sqlx::query(&sql)
            .bind(vec![id_byte; 32])
            .bind(uid)
            .execute(&app.pool)
            .await
            .unwrap();
    }

    let (sessions, tokens) = purge_expired_auth_rows(&app.pool).await.unwrap();
    assert_eq!(sessions, 1, "exactly the 8-day-expired session is purged");
    assert_eq!(tokens, 0, "no auth tokens were eligible");

    let remaining: i64 = sqlx::query_scalar!(
        r#"select count(*) as "n!" from sessions where user_id = $1"#,
        uid
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    // login session + (b) + (c)
    assert_eq!(remaining, 3);
    let purged_gone: bool = sqlx::query_scalar!(
        r#"select not exists(select 1 from sessions where id = $1) as "gone!""#,
        &vec![1u8; 32]
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert!(purged_gone, "the 8-day-expired session row must be deleted");
}

#[tokio::test]
async fn purge_deletes_month_old_verification_tokens_keeps_fresh() {
    let app = TestApp::launch().await;
    let (_cookie, uid) = app
        .signup_with_handle("Token User", "tokenuser", "tokens@example.com")
        .await;

    // Signup itself issued one fresh email-verification token; that and
    // the explicit fresh rows below must survive. Age-based deletion is
    // on created_at only (the tables double as rate-limit logs).
    sqlx::query!(
        "insert into email_verification_tokens (token_hash, user_id, expires_at, created_at)
         values ($1, $2, now() - interval '30 days', now() - interval '31 days')",
        &[1u8; 32][..],
        uid
    )
    .execute(&app.pool)
    .await
    .unwrap();
    sqlx::query!(
        "insert into email_verification_tokens (token_hash, user_id, expires_at, created_at)
         values ($1, $2, now() + interval '1 day', now() - interval '1 day')",
        &[2u8; 32][..],
        uid
    )
    .execute(&app.pool)
    .await
    .unwrap();
    sqlx::query!(
        "insert into password_reset_tokens (token_hash, user_id, expires_at, created_at)
         values ($1, $2, now() - interval '30 days', now() - interval '45 days')",
        &[3u8; 32][..],
        uid
    )
    .execute(&app.pool)
    .await
    .unwrap();
    sqlx::query!(
        "insert into password_reset_tokens (token_hash, user_id, expires_at, created_at)
         values ($1, $2, now() + interval '1 hour', now())",
        &[4u8; 32][..],
        uid
    )
    .execute(&app.pool)
    .await
    .unwrap();

    let (sessions, tokens) = purge_expired_auth_rows(&app.pool).await.unwrap();
    assert_eq!(sessions, 0, "the login session is fresh");
    assert_eq!(
        tokens, 2,
        "exactly the 31-day and 45-day-old tokens are purged"
    );

    let ev_left: i64 = sqlx::query_scalar!(
        r#"select count(*) as "n!" from email_verification_tokens where user_id = $1"#,
        uid
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    // signup-issued token + the fresh hand-inserted one
    assert_eq!(ev_left, 2, "fresh verification tokens must survive");

    let pr_left: i64 = sqlx::query_scalar!(
        r#"select count(*) as "n!" from password_reset_tokens where user_id = $1"#,
        uid
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(pr_left, 1, "fresh reset token must survive");
}
