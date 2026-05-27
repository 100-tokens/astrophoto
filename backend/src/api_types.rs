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
    /// URL handle — used to build /u/<handle> profile URLs from the
    /// session user without a second round-trip to look it up.
    pub handle: String,
    pub created_at: String,
    pub following_ids: Vec<String>,
    pub pending_deletion_at: Option<String>, // RFC3339, present only when scheduled
    pub tier: UserTier,
}

#[derive(Debug, Serialize, Deserialize, TS, PartialEq, Eq, Clone, Copy)]
#[ts(export, export_to = "UserTier.ts")]
#[serde(rename_all = "lowercase")]
pub enum UserTier {
    Free,
    Subscriber,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "AuthError.ts")]
pub struct AuthError {
    pub error: String,
    pub message: String,
}

/// Single row in the /api/photographers index. Has just enough to render
/// a tile (name + handle + cover photo) plus the headline stats users
/// sort against (frames, followers, integration).
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "PhotographerListItem.ts")]
pub struct PhotographerListItem {
    pub handle: String,
    pub display_name: String,
    pub frame_count: i64,
    pub follower_count: i64,
    pub integration_seconds: i64,
    pub cover_photo_id: Option<String>,
    pub member_since_year: i32,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "PhotographerIndexPage.ts")]
pub struct PhotographerIndexPage {
    pub items: Vec<PhotographerListItem>,
    pub next_cursor: Option<String>,
}

/// Global counts for the home-page hero band. Cached aggressively at
/// the CDN edge; the numbers move slowly (an upload changes one).
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "SiteStats.ts")]
pub struct SiteStats {
    pub practitioners: i64,
    pub frames: i64,
    pub integration_seconds: i64,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "StorageSummary.ts")]
pub struct StorageSummary {
    /// Sum of `photos.bytes` for rows owned by the caller. Doesn't
    /// include thumbnails or display masters.
    pub used_bytes: i64,
    /// Tier-derived ceiling, used by the upload page footer to render
    /// "STORAGE · 1.84 / 5.00 GB USED". Soft signal only — per-file
    /// size enforcement lives in the upload-init handler.
    pub quota_bytes: i64,
    pub tier: UserTier,
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
    /// Equipment setup link + per-photo focal modifier (migration 0017).
    pub setup_id: Option<String>,
    pub focal_modifier: Option<String>,
    pub tags: Vec<String>,
    /// Legacy comma-joined filter names cache (photos.filters column).
    pub filters: Option<String>,
    /// Typed filter chips joined from photo_filters (migration 0018).
    pub filter_items: Vec<PhotoFilterChip>,
    /// Per-filter integration breakdown (migration 0025).
    pub filter_integrations: Vec<FilterIntegration>,
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
    pub id: String,
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

/// Same-brand sibling of an equipment item. Brand prefix = first
/// whitespace-delimited token of `canonical_name` (e.g. "antlia" for
/// "Antlia 3nm Hα Pro"). Items whose canonical_name has no space are
/// considered brand-less and produce no siblings.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "EquipmentSibling.ts")]
pub struct EquipmentSibling {
    pub kind: String,
    pub slug: String,
    pub display_name: String,
    pub usage_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "EquipmentPage.ts")]
pub struct EquipmentPage {
    pub equipment: EquipmentMeta,
    pub paired: Vec<EquipmentPaired>,
    /// Same-brand siblings (same kind, canonical_name shares the
    /// leading brand token). Sorted by usage_count desc. Up to 6 items.
    pub siblings: Vec<EquipmentSibling>,
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

/// One canonical equipment item (used inside a setup).
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "EquipmentItemRef.ts")]
pub struct EquipmentItemRef {
    pub id: String,
    pub kind: String, // 'telescope'|'camera'|'mount'|'filter'|'focal_modifier'|'guiding'
    pub canonical_name: String,
    pub display_name: String,
    /// Catalog v2 (migration 0022): structured brand/model/variant on
    /// the shared header. `brand=""` denotes an unknown brand.
    pub brand: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

/// One member of a setup (link between a setup and a canonical item).
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "SetupItem.ts")]
pub struct SetupItem {
    pub role: String, // 'optical_tube'|'focal_modifier'|'main_camera'|'mount'|'filter'
    pub item: EquipmentItemRef,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "RoleCount.ts")]
pub struct RoleCount {
    pub role: String,
    pub count: i64,
}

/// Compact list-view summary — metadata + counts per role.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "SetupSummary.ts")]
pub struct SetupSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub is_remote: bool,
    pub is_default: bool,
    pub guiding: Option<String>,
    /// "overwrite" | "fill_empty"
    pub default_apply_mode: String,
    pub updated_at: String, // RFC3339
    /// One entry per role with at least one item.
    pub item_counts: Vec<RoleCount>,
}

/// Detail-view setup with full item expansion.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "SetupDetail.ts")]
pub struct SetupDetail {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub is_remote: bool,
    pub is_default: bool,
    pub guiding: Option<String>,
    /// "overwrite" | "fill_empty"
    pub default_apply_mode: String,
    pub created_at: String,
    pub updated_at: String,
    pub items: Vec<SetupItem>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "SetupInputItem.ts")]
pub struct SetupInputItem {
    pub role: String,
    pub item_id: String,
}

/// Body for POST/PATCH /api/equipment/setups[/:id]. Items replace-all
/// on PATCH (no merge). Unknown item_ids → 422.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "SetupInput.ts")]
pub struct SetupInput {
    pub name: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub is_remote: bool,
    pub is_default: bool,
    pub guiding: Option<String>,
    /// "overwrite" | "fill_empty"
    pub default_apply_mode: String,
    pub items: Vec<SetupInputItem>,
}

/// Body for POST /api/equipment/items resolve-or-create.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "EquipmentItemInput.ts")]
pub struct EquipmentItemInput {
    pub kind: String,
    pub display_name: String,
    /// Optional per-kind specs to persist alongside the item row.
    /// If present the `specs.kind` discriminator must match `kind` or the
    /// request is rejected with 422. If absent the item is created (or
    /// resolved) without touching any `<kind>_specs` table.
    #[serde(default)]
    pub specs: Option<EquipmentSpecsPayload>,
    /// Catalog v2 (migration 0022): structured brand/model/variant.
    /// All three are optional to preserve back-compat with callers that
    /// only send `{ kind, display_name }` — when absent, the handler
    /// derives brand="" and model=trim(display_name) (the freetext
    /// fallback). When present, they take precedence and the handler
    /// regenerates display_name + canonical_name from them.
    #[serde(default)]
    pub brand: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub variant: Option<String>,
}

/// Body for POST /api/photos/:id/apply-setup.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "ApplySetupInput.ts")]
pub struct ApplySetupInput {
    pub setup_id: String,
    /// "fill_empty" | "overwrite"
    pub mode: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "DraftListItem.ts")]
pub struct DraftListItem {
    pub id: String,
    pub short_id: String,
    pub original_name: String,
    pub target: Option<String>,
    pub status: String,
    pub created_at: String,
    /// CDN URL for a small thumbnail.
    pub thumb_url: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "DraftListResponse.ts")]
pub struct DraftListResponse {
    pub items: Vec<DraftListItem>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "BatchApplyRequest.ts")]
pub struct BatchApplyRequest {
    pub ids: Vec<Uuid>,
    pub target: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "BatchApplyResponse.ts")]
pub struct BatchApplyResponse {
    pub applied: u32,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "BatchPublishRequest.ts")]
pub struct BatchPublishRequest {
    pub ids: Vec<Uuid>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "PublishedItem.ts")]
pub struct PublishedItem {
    pub id: String,
    pub short_id: String,
}

#[derive(Debug, Serialize, Deserialize, TS, Clone, Copy, PartialEq, Eq)]
#[ts(export, export_to = "SkipReason.ts")]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    StillProcessing,
    Failed,
    AlreadyPublished,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "SkippedItem.ts")]
pub struct SkippedItem {
    pub id: String,
    pub reason: SkipReason,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "BatchPublishResponse.ts")]
pub struct BatchPublishResponse {
    pub published: Vec<PublishedItem>,
    pub skipped: Vec<SkippedItem>,
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
    /// UUID of the photo — used to build CDN URLs via `/cdn/img/<photo_id>`.
    pub photo_id: String,
    /// Short human-readable ID — used to build permalink URLs `/u/<handle>/p/<short_id>`.
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

// ============================================================
// Equipment catalog — typed specs per kind. See spec
// docs/superpowers/specs/2026-05-14-equipment-catalog-enriched-design.md.
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "FilterType.ts")]
#[serde(rename_all = "snake_case")]
pub enum FilterType {
    Luminance,
    Red,
    Green,
    Blue,
    HAlpha,
    Oiii,
    Sii,
    UvIrCut,
    DualBand,
    TriBand,
    QuadBand,
    LightPollution,
    BroadbandColor,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "TelescopeDesign.ts")]
#[serde(rename_all = "snake_case")]
pub enum TelescopeDesign {
    RefractorApo,
    RefractorAchro,
    Sct,
    Rc,
    Newtonian,
    MaksutovCassegrain,
    MaksutovNewtonian,
    DallKirkham,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "CameraSensorType.ts")]
#[serde(rename_all = "lowercase")]
pub enum CameraSensorType {
    Cmos,
    Ccd,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "CameraColorType.ts")]
#[serde(rename_all = "lowercase")]
pub enum CameraColorType {
    Mono,
    Osc,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "FilterSize.ts")]
pub enum FilterSize {
    #[serde(rename = "1_25in")]
    In1_25,
    #[serde(rename = "2in")]
    In2,
    #[serde(rename = "31mm")]
    Mm31,
    #[serde(rename = "36mm")]
    Mm36,
    #[serde(rename = "50mm_round")]
    Mm50Round,
    #[serde(rename = "50mm_square")]
    Mm50Square,
    #[serde(rename = "other")]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "MountType.ts")]
#[serde(rename_all = "snake_case")]
pub enum MountType {
    EquatorialGerman,
    EquatorialFork,
    AltAz,
    HarmonicDrive,
    StrainWave,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "FocalModifierType.ts")]
#[serde(rename_all = "snake_case")]
pub enum FocalModifierType {
    Reducer,
    Flattener,
    ReducerFlattener,
    Barlow,
    Extender,
    Corrector,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, export_to = "GuidingSetupKind.ts")]
#[serde(rename_all = "snake_case")]
pub enum GuidingSetupKind {
    Oag,
    Guidescope,
    OagPrism,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "TelescopeSpecs.ts")]
pub struct TelescopeSpecs {
    pub design: Option<TelescopeDesign>,
    pub aperture_mm: Option<i32>,
    pub focal_length_mm: Option<i32>,
    /// Computed (DB-generated). Returned in GET, ignored in PATCH/POST.
    #[serde(default)]
    pub focal_ratio_f: Option<f64>,
    /// Catalog v2 (migration 0022): completeness fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub self_weight_kg: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optical_length_mm: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backfocus_mm: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "CameraSpecs.ts")]
pub struct CameraSpecs {
    pub sensor_type: Option<CameraSensorType>,
    pub color_type: Option<CameraColorType>,
    pub cooled: Option<bool>,
    pub sensor_model: Option<String>,
    pub pixel_size_um: Option<f64>,
    pub sensor_width_px: Option<i32>,
    pub sensor_height_px: Option<i32>,
    /// Catalog v2 (migration 0022): completeness fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub self_weight_g: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_well_capacity_e: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_noise_e: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mount_thread: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backfocus_mm: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "FilterSpecs.ts")]
pub struct FilterSpecs {
    pub filter_type: Option<FilterType>,
    pub bandwidth_nm: Option<f64>,
    pub size: Option<FilterSize>,
    pub mounted: Option<bool>,
    /// Catalog v2 (migration 0022): completeness fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mounted_diameter_mm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thickness_mm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peak_transmission_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "MountSpecs.ts")]
pub struct MountSpecs {
    pub mount_type: Option<MountType>,
    pub payload_kg: Option<f64>,
    pub goto: Option<bool>,
    /// Catalog v2 (migration 0022): completeness fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub self_weight_kg: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub periodic_error_arcsec: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tripod_included: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_protocol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "FocalModifierSpecs.ts")]
pub struct FocalModifierSpecs {
    pub modifier_type: Option<FocalModifierType>,
    pub factor: Option<f64>,
    /// Catalog v2 (migration 0022): completeness fields.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub self_weight_g: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backfocus_mm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_circle_mm: Option<f64>,
}

/// Catalog v2 (migration 0022): guiding equipment spec sub-table.
/// `setup_kind` is required (the DB CHECK enforces it); the rest are
/// optional just like the other spec structs.
#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "GuidingSpecs.ts")]
pub struct GuidingSpecs {
    pub setup_kind: Option<GuidingSetupKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guide_focal_mm: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guide_aperture_mm: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guide_camera: Option<String>,
}

/// Tagged union — `kind` discriminator on the wire matches `equipment_items.kind`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "EquipmentSpecsPayload.ts")]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EquipmentSpecsPayload {
    Telescope(TelescopeSpecs),
    Camera(CameraSpecs),
    Filter(FilterSpecs),
    Mount(MountSpecs),
    FocalModifier(FocalModifierSpecs),
    /// Catalog v2 (migration 0022): typed specs for guiding equipment.
    Guiding(GuidingSpecs),
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "EquipmentItemPatch.ts")]
pub struct EquipmentItemPatch {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub specs: Option<EquipmentSpecsPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "EquipmentItemDetail.ts")]
pub struct EquipmentItemDetail {
    pub id: String,
    pub kind: String,
    pub canonical_name: String,
    pub display_name: String,
    pub usage_count: i32,
    pub status: String,
    pub submitted_by: Option<String>,
    pub approved_at: Option<String>,
    pub created_at: String,
    pub specs: Option<EquipmentSpecsPayload>,
    /// Catalog v2 (migration 0022): structured brand/model/variant on
    /// the shared header. `brand=""` denotes an unknown brand (typically
    /// a freetext-created row that hasn't been moderated). `model` is
    /// always populated.
    pub brand: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    /// Submitter handle, surfaced for the detail page's "added by
    /// @handle" footer. None when `submitted_by` is null OR when the
    /// submitter has been deleted (FK is ON DELETE SET NULL upstream).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub submitted_by_handle: Option<String>,
    /// Number of distinct `equipment_setups` referencing this item via
    /// `setup_items`. Drives the detail-page stat strip and the
    /// "Delete" affordance (only shown when zero setups + zero photos).
    #[serde(default)]
    pub setup_count: i64,
}

// ============================================================
// Catalog browse — facet counts + filtered/sorted item list.
// Returned by GET /api/equipment/catalog?kind=... — used by the
// `/equip/[kind]` browse page.
// ============================================================

/// One facet bucket: a category value and the number of items in the
/// catalog (for the given kind) that have that value.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "EquipmentFacetBucket.ts")]
pub struct EquipmentFacetBucket {
    pub value: String,
    pub count: i64,
}

/// Sidebar facets for the browse page. Each per-kind enum facet stays
/// `Some(_)` only when the kind has that field — irrelevant facets are
/// omitted so the frontend can branch on `Option::is_some()` rather
/// than counting empty vectors.
#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[ts(export, export_to = "EquipmentFacets.ts")]
pub struct EquipmentFacets {
    pub brands: Vec<EquipmentFacetBucket>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub designs: Option<Vec<EquipmentFacetBucket>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensor_types: Option<Vec<EquipmentFacetBucket>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_types: Option<Vec<EquipmentFacetBucket>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooled: Option<Vec<EquipmentFacetBucket>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mount_types: Option<Vec<EquipmentFacetBucket>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter_types: Option<Vec<EquipmentFacetBucket>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modifier_types: Option<Vec<EquipmentFacetBucket>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setup_kinds: Option<Vec<EquipmentFacetBucket>>,
}

/// Page response for the browse endpoint. Items are fully expanded
/// (joined to their per-kind spec table) so the grid card can render
/// the right summary line without a second round trip.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "EquipmentCatalogResponse.ts")]
pub struct EquipmentCatalogResponse {
    pub items: Vec<EquipmentItemDetail>,
    pub facets: EquipmentFacets,
    pub total: i64,
    /// Page size used to compute `total / limit` pagination on the
    /// frontend. Mirrors the request param (clamped server-side).
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "PhotoFilterChip.ts")]
pub struct PhotoFilterChip {
    pub id: String,
    pub display_name: String,
    pub filter_type: Option<FilterType>,
    pub bandwidth_nm: Option<f64>,
    pub position: i32,
}

/// One per-filter integration row: how many sub-frames at what exposure,
/// for a given filter. Display-only acquisition detail stored as a JSONB
/// list on `photos.filter_integrations` (independent of the filter chips,
/// so luminance — which is not a chip — is representable). Per-filter and
/// grand totals are derived client-side (`sub_count * sub_exposure_s`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "FilterIntegration.ts")]
pub struct FilterIntegration {
    /// Filter band ALIAS / display label: "L" | "R" | "G" | "B" | "Ha" |
    /// "OIII" | "SII" | free text. Read from the master's FITS FILTER
    /// keyword; this is a short code, not a catalog reference.
    pub filter: String,
    /// Number of integrated sub-frames. >= 0.
    pub sub_count: i32,
    /// Per-sub exposure in seconds. >= 0.
    pub sub_exposure_s: f64,
    /// Optional link to a real catalog filter (`equipment_items.id`,
    /// kind='filter'). The header `filter` is only an alias; the user
    /// reconciles it to a catalog item (auto-matched on drop, overridable).
    /// `None` when the band has no catalog entry — e.g. Luminance for a
    /// photographer who keeps no L filter in their catalog.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter_item_id: Option<String>,
    /// Per-session camera gain (unitless ZWO/CMOS setting, FITS `GAIN`).
    /// Session-specific, so it lives here rather than the global
    /// `photos.gain` column — R and Hα are often shot at different gains.
    /// `None` when unknown. See `docs/superpowers/specs/2026-05-27-
    /// acquisition-session-fields-design.md`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gain: Option<i32>,
    /// Per-session sensor temperature in °C (FITS `CCD-TEMP`, actual,
    /// preferred over `SET-TEMP`). Nullable; legitimately negative.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensor_temp_c: Option<f64>,
}
