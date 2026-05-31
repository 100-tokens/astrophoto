//! Super-admin API surface (`/api/admin/*`).
//!
//! Every handler is guarded by the [`crate::auth::middleware::AdminUser`]
//! extractor: `401` for anonymous requests, `403` for authenticated
//! non-admins. Scope (deliberately bounded): manage the equipment catalog
//! and the runtime app settings. Broader moderation (users, photos, audit
//! logs) is intentionally out of scope.
pub mod equipment;
pub mod settings;
