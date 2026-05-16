//! Equipment setups: per-user reusable gear bundles. See
//! docs/superpowers/specs/2026-05-04-equipment-setups-design.md.

pub mod create;
pub mod delete;
pub mod get;
pub mod list;
pub mod update;

const VALID_ROLES: &[&str] = &[
    "optical_tube",
    "focal_modifier",
    "main_camera",
    "mount",
    "filter",
];

pub fn validate_role(role: &str) -> Result<(), crate::error::AppError> {
    if VALID_ROLES.contains(&role) {
        Ok(())
    } else {
        Err(crate::error::AppError::Validation(format!(
            "unknown role '{role}'"
        )))
    }
}

/// Accepted values for `equipment_setups.default_apply_mode`. The DB has
/// a matching CHECK constraint (migration 0019) so this validator is
/// defence-in-depth — we surface a 422 with a clear message instead of
/// bubbling up a database error.
const VALID_APPLY_MODES: &[&str] = &["overwrite", "fill_empty"];

pub fn validate_default_apply_mode(mode: &str) -> Result<(), crate::error::AppError> {
    if VALID_APPLY_MODES.contains(&mode) {
        Ok(())
    } else {
        Err(crate::error::AppError::Validation(format!(
            "default_apply_mode must be 'overwrite' or 'fill_empty', got '{mode}'"
        )))
    }
}

pub(super) fn unique_conflict_to_422(e: sqlx::Error) -> crate::error::AppError {
    if let Some(db) = e.as_database_error()
        && db.code().as_deref() == Some("23505")
    {
        return crate::error::AppError::Validation("a setup with this name already exists".into());
    }
    e.into()
}

pub(super) fn unknown_item_to_422(e: sqlx::Error) -> crate::error::AppError {
    if let Some(db) = e.as_database_error()
        && db.code().as_deref() == Some("23503")
    {
        return crate::error::AppError::Validation("unknown item_id".into());
    }
    e.into()
}
