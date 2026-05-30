//! Per-account login brute-force throttle.
//!
//! Keyed on `user_id` — never on IP. Behind our reverse proxy the client IP
//! collapses to a single egress address (the same lesson that reshaped the
//! password-reset throttle), so an IP axis is either useless or a global DoS
//! lever. The account axis is not spoofable.
//!
//! Model (see `migrations/0027_login_throttle.sql` for the schema rationale):
//!   - Each wrong password upserts the user's row, incrementing `failed_count`.
//!   - When `failed_count` first reaches [`MAX_FAILURES`], a FIXED `locked_until`
//!     is stamped (`now() + LOCKOUT`) and the counter resets to 0.
//!   - While locked, the handler short-circuits ([`is_locked`]) BEFORE running
//!     Argon2, so no further failure is recorded and the lock is never extended
//!     — the lockout is a bounded, known-duration speed bump, not something an
//!     attacker can sustain by continuing to guess.
//!   - A successful login [`clear`]s the row, so a legitimate user is never
//!     more than one lock window away from getting back in.

use crate::AppError;

/// Failed attempts (within a single lock cycle) before the account locks.
pub const MAX_FAILURES: i32 = 10;
/// How long an account stays locked once the threshold is crossed.
pub const LOCKOUT_MINUTES: i32 = 15;

/// True if the account currently has an active lock. Cheap single-row read run
/// before any Argon2 work.
pub async fn is_locked(pool: &sqlx::PgPool, user_id: uuid::Uuid) -> Result<bool, AppError> {
    let locked = sqlx::query_scalar!(
        "select exists(
            select 1 from login_throttle
             where user_id = $1 and locked_until is not null and locked_until > now()
        )",
        user_id
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(false);
    Ok(locked)
}

/// Record one failed login. Increments the counter; when it crosses
/// [`MAX_FAILURES`] it stamps a fixed `locked_until` and resets the counter to
/// 0 (so the next lock requires a fresh batch of failures). The `locked_until`
/// is only set when not already locked, so a lock is never extended. Only ever
/// called for a NOT-currently-locked account (the handler short-circuits locked
/// ones first).
pub async fn record_failure(pool: &sqlx::PgPool, user_id: uuid::Uuid) -> Result<(), AppError> {
    sqlx::query!(
        "insert into login_throttle (user_id, failed_count, locked_until, updated_at)
              values ($1, 1, null, now())
         on conflict (user_id) do update set
             failed_count = case
                 when login_throttle.failed_count + 1 >= $2 then 0
                 else login_throttle.failed_count + 1
             end,
             locked_until = case
                 when login_throttle.failed_count + 1 >= $2
                      and (login_throttle.locked_until is null
                           or login_throttle.locked_until <= now())
                 then now() + make_interval(mins => $3)
                 else login_throttle.locked_until
             end,
             updated_at = now()",
        user_id,
        MAX_FAILURES,
        LOCKOUT_MINUTES,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Clear all throttle state for a user after a successful login.
pub async fn clear(pool: &sqlx::PgPool, user_id: uuid::Uuid) -> Result<(), AppError> {
    sqlx::query!("delete from login_throttle where user_id = $1", user_id)
        .execute(pool)
        .await?;
    Ok(())
}
