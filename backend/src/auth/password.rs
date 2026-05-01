//! Argon2id password hashing helpers.
//!
//! Verification and hashing are CPU-bound, so both APIs run inside
//! `tokio::task::spawn_blocking`. Direct synchronous variants (suffixed `_blocking`)
//! exist for unit tests.

use argon2::{
    Argon2, PasswordHash,
    password_hash::{PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

use crate::AppError;

/// Hash a plaintext password. Async-safe — runs the work on the blocking pool.
pub async fn hash(password: String) -> Result<String, AppError> {
    tokio::task::spawn_blocking(move || hash_blocking(&password))
        .await
        .map_err(|e| AppError::Internal(format!("argon2 join: {e}")))?
}

/// Verify a plaintext password against a stored hash.
pub async fn verify(password: String, hash_str: String) -> Result<bool, AppError> {
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
