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
