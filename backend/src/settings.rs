//! Runtime-editable, app-wide settings (managed by super-admins).
//!
//! Backed by the single-row `app_settings` table (migration 0032). The reader
//! is **fail-safe by contract**: a missing row or any DB error logs a warning
//! and returns the compile-time defaults below. Settings are read on the
//! signup and upload hot paths, so a settings hiccup must never break them.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use ts_rs::TS;

use crate::AppError;

/// Compile-time fallbacks — must match the column defaults in migration 0032
/// and the historical hardcoded constants they replaced.
pub const DEFAULT_SIGNUPS_ENABLED: bool = true;
pub const DEFAULT_FREE_UPLOAD_MB: i32 = 50;
pub const DEFAULT_SUBSCRIBER_UPLOAD_MB: i32 = 200;

/// Bounds enforced on writes (mirror the CHECK constraints in migration 0032).
pub const MIN_UPLOAD_MB: i32 = 1;
pub const MAX_UPLOAD_MB: i32 = 100_000;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "AppSettings.ts")]
pub struct AppSettings {
    pub signups_enabled: bool,
    pub free_upload_max_mb: i32,
    pub subscriber_upload_max_mb: i32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            signups_enabled: DEFAULT_SIGNUPS_ENABLED,
            free_upload_max_mb: DEFAULT_FREE_UPLOAD_MB,
            subscriber_upload_max_mb: DEFAULT_SUBSCRIBER_UPLOAD_MB,
        }
    }
}

impl AppSettings {
    /// Per-file upload ceiling in bytes for the given tier string.
    pub fn upload_max_bytes(&self, tier: &str) -> u64 {
        let mb = if tier == "subscriber" {
            self.subscriber_upload_max_mb
        } else {
            self.free_upload_max_mb
        };
        (mb.max(MIN_UPLOAD_MB) as u64) * 1024 * 1024
    }
}

/// Read the singleton settings row. INFALLIBLE: a missing row or any error
/// falls back to [`AppSettings::default`] (logged), so callers never break.
pub async fn get(pool: &PgPool) -> AppSettings {
    match sqlx::query_as!(
        AppSettings,
        r#"select signups_enabled, free_upload_max_mb, subscriber_upload_max_mb
           from app_settings where id = 1"#
    )
    .fetch_optional(pool)
    .await
    {
        Ok(Some(s)) => s,
        Ok(None) => {
            tracing::warn!("app_settings row missing; using compile-time defaults");
            AppSettings::default()
        }
        Err(e) => {
            tracing::warn!(error = %e, "app_settings read failed; using compile-time defaults");
            AppSettings::default()
        }
    }
}

/// Overwrite the settings row (super-admin action). Validates bounds, upserts
/// the singleton, stamps `updated_by`, and returns the persisted values.
/// Unlike [`get`], this is fallible — admins see validation/DB errors.
pub async fn update(
    pool: &PgPool,
    new: &AppSettings,
    updated_by: uuid::Uuid,
) -> Result<AppSettings, AppError> {
    for mb in [new.free_upload_max_mb, new.subscriber_upload_max_mb] {
        if !(MIN_UPLOAD_MB..=MAX_UPLOAD_MB).contains(&mb) {
            return Err(AppError::Validation(format!(
                "upload limit must be {MIN_UPLOAD_MB}..={MAX_UPLOAD_MB} MB"
            )));
        }
    }
    let row = sqlx::query_as!(
        AppSettings,
        r#"
        insert into app_settings (id, signups_enabled, free_upload_max_mb, subscriber_upload_max_mb, updated_at, updated_by)
        values (1, $1, $2, $3, now(), $4)
        on conflict (id) do update set
            signups_enabled = excluded.signups_enabled,
            free_upload_max_mb = excluded.free_upload_max_mb,
            subscriber_upload_max_mb = excluded.subscriber_upload_max_mb,
            updated_at = now(),
            updated_by = excluded.updated_by
        returning signups_enabled, free_upload_max_mb, subscriber_upload_max_mb
        "#,
        new.signups_enabled,
        new.free_upload_max_mb,
        new.subscriber_upload_max_mb,
        updated_by,
    )
    .fetch_one(pool)
    .await?;
    Ok(row)
}
