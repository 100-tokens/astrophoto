//! Celestial-object identification (D5/D6 from celestial-objects spec).
//! Cone search + write to photo_targets at plate-solve time.
//!
//! Submodules are added incrementally — see plan
//! `docs/superpowers/plans/2026-05-28-celestial-identify-overlay-plan.md`.

pub mod confidence;
pub mod queries;
