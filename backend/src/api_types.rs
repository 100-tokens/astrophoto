//! Types exported to the frontend via ts-rs.
//! Mirror DTOs only; never expose internal structs.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "Health.ts")]
pub struct Health {
    pub status: String,
    pub db: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "User.ts")]
pub struct User {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub created_at: String,
    pub following_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "AuthError.ts")]
pub struct AuthError {
    pub error: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "UserPublic.ts")]
pub struct UserPublic {
    pub id: String,
    pub display_name: String,
    pub created_at: String,
    pub photo_count: i64,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "Profile.ts")]
pub struct Profile {
    pub display_name: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "Preferences.ts")]
pub struct Preferences {
    pub theme: String,
    pub density: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "SessionRow.ts")]
pub struct SessionRow {
    pub id: String,
    pub browser: String,
    pub browser_version: String,
    pub os: String,
    pub os_version: String,
    pub category: String,
    pub ip: String,
    pub last_used_at: String, // RFC3339
    pub created_at: String,
    pub is_current: bool,
}
