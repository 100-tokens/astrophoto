//! Types exported to the frontend via ts-rs.
//! Mirror DTOs only; never expose internal structs.

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

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
    /// URL-safe handle (e.g. `marie`). Used to build canonical
    /// `/u/<handle>` URLs from a known user id.
    pub handle: String,
    pub display_name: String,
    pub created_at: String,
    pub photo_count: i64,
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
    /// Migration 0013: extended acquisition fields surfaced on the verify form.
    pub aperture_f: Option<f32>,
    pub gain: Option<i16>,
    pub sensor_temp_c: Option<f32>,
    pub sessions: Option<i16>,
    /// Plate-solving result. Today filled manually on the verify form;
    /// a future plate-solve job will populate these from astrometry.
    pub ra_deg: Option<f64>,
    pub dec_deg: Option<f64>,
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

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq, Hash)]
#[ts(export, export_to = "SocialPlatform.ts")]
#[serde(rename_all = "snake_case")]
pub enum SocialPlatform {
    Twitter,
    Instagram,
    Bluesky,
    Astrobin,
    Mastodon,
    Youtube,
    Website,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq, Hash)]
#[ts(export, export_to = "SocialLink.ts")]
pub struct SocialLink {
    pub platform: SocialPlatform,
    pub url: String,
}

// PartialEq/Eq/Hash on SocialLink so the validator can detect duplicate platforms.
// (Hash is on SocialLink itself though we only really need it on SocialPlatform —
// adding it everywhere is cheap and keeps signatures simple.)

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "EquipmentSummary.ts")]
pub struct EquipmentSummary {
    pub telescope: Option<String>,
    pub camera: Option<String>,
    pub mount: Option<String>,
    pub filters: Option<String>,
    pub guiding: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "LocationSummary.ts")]
pub struct LocationSummary {
    pub location_text: Option<String>,
    pub bortle_class: Option<i16>,
    pub sqm: Option<f64>,
}

/// Authenticated owner's view of their own profile (writable surface).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "Profile.ts")]
pub struct Profile {
    pub display_name: String,
    pub tagline: Option<String>,
    pub bio_html: Option<String>,
    pub cover_photo_id: Option<Uuid>,
    pub equipment: EquipmentSummary,
    pub location: LocationSummary,
    pub social_links: Vec<SocialLink>,
}

/// Patch body — every field is optional; absent = leave alone, explicit null = clear.
/// `ProfilePatch` is NOT exported to TS — frontend builds its own partial type.
#[derive(Debug, Clone, Deserialize)]
pub struct ProfilePatch {
    #[serde(default, deserialize_with = "deserialize_some")]
    pub display_name: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub tagline: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub bio_html: Option<Option<String>>,
    pub equipment: Option<EquipmentSummary>,
    pub location: Option<LocationSummary>,
    pub social_links: Option<Vec<SocialLink>>,
}

/// Helper: distinguishes "field absent" from "field present and null" for double-Option.
fn deserialize_some<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "HeroStats.ts")]
pub struct HeroStats {
    pub frames: i64,
    pub integration_seconds: i64,
    pub followers: i64,
    pub appreciations: i64,
    pub targets: i64,
    pub member_since_year: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "FeaturedPhotoSummary.ts")]
pub struct FeaturedPhotoSummary {
    pub id: Uuid,
    pub short_id: String,
    pub featured_position: i16,
    pub target: Option<String>,
    pub appreciations_count: i32,
    pub blurhash: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

/// Public read-side aggregator returned by GET /api/users/by-handle/:handle/profile.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "PublicProfile.ts")]
pub struct PublicProfile {
    pub id: Uuid,
    pub handle: String,
    pub display_name: String,
    pub tagline: Option<String>,
    pub bio_html: Option<String>,
    pub cover: Option<FeaturedPhotoSummary>,
    pub equipment: EquipmentSummary,
    pub location: LocationSummary,
    pub social_links: Vec<SocialLink>,
    pub stats: HeroStats,
    pub featured: Vec<FeaturedPhotoSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "GalleryPhoto.ts")]
pub struct GalleryPhoto {
    pub id: Uuid,
    pub short_id: String,
    pub target: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub blurhash: Option<String>,
    pub appreciations_count: i32,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "GalleryPage.ts")]
pub struct GalleryPage {
    pub photos: Vec<GalleryPhoto>,
    /// Opaque cursor; pass back as `?cursor=` to load the next page. `None` when exhausted.
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "DiscoveryPhoto.ts")]
pub struct DiscoveryPhoto {
    pub id: Uuid,
    pub short_id: String,
    pub target: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub blurhash: Option<String>,
    pub appreciations_count: i32,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    pub author_id: Uuid,
    pub author_handle: String,
    pub author_display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "DiscoveryPage.ts")]
pub struct DiscoveryPage {
    pub photos: Vec<DiscoveryPhoto>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "TargetMeta.ts")]
pub struct TargetMeta {
    pub slug: String,
    pub canonical_name: String,
    pub aliases: Vec<String>,
    pub kind: Option<String>,
    pub photo_count: i64,
    pub contributor_count: i64,
    // — additions D2b —
    pub right_ascension: Option<f64>,
    pub declination: Option<f64>,
    pub magnitude_v: Option<f32>,
    pub object_type: Option<String>,
    pub constellation: Option<String>,
    pub major_axis_arcmin: Option<f32>,
    pub minor_axis_arcmin: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "TargetPage.ts")]
pub struct TargetPage {
    pub target: TargetMeta,
    pub page: DiscoveryPage,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "TagMeta.ts")]
pub struct TagMeta {
    pub slug: String,
    pub name: String,
    pub photo_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "TagPage.ts")]
pub struct TagPage {
    pub tag: TagMeta,
    pub page: DiscoveryPage,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "EquipmentMeta.ts")]
pub struct EquipmentMeta {
    pub kind: String,
    pub slug: String,
    pub canonical_name: String,
    pub display_name: String,
    pub photo_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "EquipmentPaired.ts")]
pub struct EquipmentPaired {
    pub kind: String,
    pub slug: String,
    pub display_name: String,
    pub shared_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "EquipmentPage.ts")]
pub struct EquipmentPage {
    pub equipment: EquipmentMeta,
    pub paired: Vec<EquipmentPaired>,
    pub page: DiscoveryPage,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "CategoryPage.ts")]
pub struct CategoryPage {
    pub category: String,
    pub photo_count: i64,
    pub page: DiscoveryPage,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "SearchTargetHit.ts")]
pub struct SearchTargetHit {
    pub slug: String,
    pub canonical_name: String,
    pub photo_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "SearchUserHit.ts")]
pub struct SearchUserHit {
    pub id: Uuid,
    pub handle: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "SearchResults.ts")]
pub struct SearchResults {
    pub q: String,
    pub targets: Vec<SearchTargetHit>,
    pub users: Vec<SearchUserHit>,
    pub photos: Vec<DiscoveryPhoto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "PatchTargetsItem.ts")]
pub struct PatchTargetsItem {
    pub slug: String,
    pub canonical_name: String,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "PatchTargetsResponse.ts")]
pub struct PatchTargetsResponse {
    pub targets: Vec<PatchTargetsItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "TargetPreviewThumb.ts")]
pub struct TargetPreviewThumb {
    pub short_id: String,
    pub blurhash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "TargetListItem.ts")]
pub struct TargetListItem {
    pub slug: String,
    pub canonical_name: String,
    pub object_type: Option<String>,
    pub constellation: Option<String>,
    pub magnitude_v: Option<f32>,
    pub photo_count: i64,
    pub preview_thumbs: Vec<TargetPreviewThumb>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "TargetIndexPage.ts")]
pub struct TargetIndexPage {
    pub targets: Vec<TargetListItem>,
    pub next_cursor: Option<String>,
}
