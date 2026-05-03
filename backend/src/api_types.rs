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
    pub pending_deletion_at: Option<String>, // RFC3339, present only when scheduled
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

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "MeStats.ts")]
pub struct MeStats {
    pub published_count: i64,
    pub draft_count: i64,
    pub integration_secs: f64,
    pub appreciations_received: i64,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "PhotoDetail.ts")]
pub struct PhotoDetail {
    pub id: String,
    pub owner_id: String,
    pub short_id: String,
    pub status: String,
    pub original_name: String,
    pub bytes: i64,
    pub mime: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub iso: Option<i32>,
    pub exposure_s: Option<f64>,
    pub focal_mm: Option<f64>,
    pub target: Option<String>,
    pub caption: Option<String>,
    pub taken_at: Option<String>,
    pub created_at: String,
    pub appreciation_count: i64,
    pub comment_count: i64,
    pub is_draft: bool,
    pub last_step: Option<String>,
    pub replaced_at: Option<String>,
    pub original_uploaded_at: String,
    pub pipeline_error: Option<String>,
}
