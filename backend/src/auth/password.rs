//! Argon2id password hashing helpers.
//!
//! Verification and hashing are CPU-bound, so both APIs run inside
//! `tokio::task::spawn_blocking`. Direct synchronous variants (suffixed `_blocking`)
//! exist for unit tests.

use std::sync::LazyLock;
use std::time::Duration;

use argon2::{
    Argon2, PasswordHash,
    password_hash::{PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use tokio::sync::{Semaphore, SemaphorePermit};

use crate::AppError;

const COMMON: &str = include_str!("../../assets/common-passwords.txt");

/// Cap on concurrent Argon2id operations. Argon2id is deliberately CPU- and
/// memory-hard, and every call runs on the blocking pool. Without a bound, a
/// burst of auth requests (login, signup, reset-confirm) each spawn a blocking
/// task and can saturate every core — and balloon the blocking-thread pool —
/// starving the rest of the API. The per-account login throttle sheds the bulk
/// of a single-account brute force before it reaches Argon2; this semaphore is
/// the second line, bounding a distributed flood across many accounts. 8
/// permits leave ample room for legitimate concurrency.
const MAX_CONCURRENT_ARGON2: usize = 8;
/// A request that cannot get a slot within this window is shed with 503 rather
/// than queueing unboundedly behind a sustained flood.
const ARGON2_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);

static ARGON2_SLOTS: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(MAX_CONCURRENT_ARGON2));

/// Acquire a concurrency slot, held for the duration of the blocking Argon2
/// work. Returns `ServiceUnavailable` (503) if the pool stays saturated past
/// the timeout — backpressure, not an unbounded waiter queue.
async fn argon2_slot() -> Result<SemaphorePermit<'static>, AppError> {
    match tokio::time::timeout(ARGON2_ACQUIRE_TIMEOUT, ARGON2_SLOTS.acquire()).await {
        Ok(Ok(permit)) => Ok(permit),
        // The semaphore is a 'static singleton and is never closed.
        Ok(Err(_)) => Err(AppError::internal("argon2 semaphore closed")),
        Err(_) => Err(AppError::ServiceUnavailable),
    }
}

/// Validate password strength. Returns `Err` with a static error code string
/// if the password is too short or appears in the common-password dictionary.
pub fn validate_strength(pwd: &str) -> Result<(), &'static str> {
    if pwd.chars().count() < 12 {
        return Err("password_too_short");
    }
    let lower = pwd.to_ascii_lowercase();
    if COMMON.lines().any(|p| p.trim() == lower) {
        return Err("password_too_common");
    }
    Ok(())
}

/// Hash a plaintext password. Async-safe — runs the work on the blocking pool,
/// gated by the Argon2 concurrency semaphore.
pub async fn hash(password: String) -> Result<String, AppError> {
    let _permit = argon2_slot().await?;
    tokio::task::spawn_blocking(move || hash_blocking(&password))
        .await
        .map_err(|e| AppError::Internal(format!("argon2 join: {e}")))?
}

/// Verify a plaintext password against a stored hash. Gated by the Argon2
/// concurrency semaphore (the permit is held across the blocking verify).
pub async fn verify(password: String, hash_str: String) -> Result<bool, AppError> {
    let _permit = argon2_slot().await?;
    tokio::task::spawn_blocking(move || verify_blocking(&password, &hash_str))
        .await
        .map_err(|e| AppError::Internal(format!("argon2 join: {e}")))?
}

fn hash_blocking(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("argon2 hash: {e}")))?;
    Ok(hash.to_string())
}

fn verify_blocking(password: &str, stored: &str) -> Result<bool, AppError> {
    let parsed =
        PasswordHash::new(stored).map_err(|e| AppError::Internal(format!("parse hash: {e}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hash_and_verify_round_trip() {
        let h = hash("correct horse battery staple".to_string())
            .await
            .unwrap();
        assert!(
            verify("correct horse battery staple".to_string(), h.clone())
                .await
                .unwrap()
        );
        assert!(!verify("wrong password".to_string(), h).await.unwrap());
    }

    #[tokio::test]
    async fn unique_hashes_each_call() {
        let h1 = hash("same".into()).await.unwrap();
        let h2 = hash("same".into()).await.unwrap();
        assert_ne!(h1, h2, "salt must be unique per call");
    }
}
