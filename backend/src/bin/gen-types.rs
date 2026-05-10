//! Emits TypeScript types to the frontend. Invoked via `just types`.

use std::fs;
use std::path::Path;
use ts_rs::TS;

use astrophoto::api_types::{
    AuthError, BatchApplyRequest, BatchApplyResponse, BatchPublishRequest, BatchPublishResponse,
    CategoryPage, DiscoveryPage, DiscoveryPhoto, DraftListItem, DraftListResponse, EquipmentMeta,
    EquipmentPage, EquipmentPaired, EquipmentSummary, FeaturedPhotoSummary, GalleryPage,
    GalleryPhoto, Health, HeroStats, LocationSummary, MeStats, PatchTargetsItem,
    PatchTargetsResponse, PhotoDetail, Preferences, Profile, PublicProfile, PublishedItem,
    SearchResults, SearchTargetHit, SearchUserHit, SessionRow, SkipReason, SkippedItem, SocialLink,
    PhotographerIndexPage, PhotographerListItem, SocialPlatform, StorageSummary, TagMeta, TagPage,
    TargetIndexPage, TargetListItem, TargetMeta, TargetPage, TargetPreviewThumb, User, UserPublic,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = Path::new("../frontend/src/lib/api");
    fs::create_dir_all(out_dir)?;

    // export_all_to writes each type (from #[ts(export_to = "Foo.ts")])
    // into the given directory, together with all transitive dependencies.
    Health::export_all_to(out_dir)?;
    User::export_all_to(out_dir)?;
    AuthError::export_all_to(out_dir)?;
    UserPublic::export_all_to(out_dir)?;
    Preferences::export_all_to(out_dir)?;
    SessionRow::export_all_to(out_dir)?;
    MeStats::export_all_to(out_dir)?;
    PhotoDetail::export_all_to(out_dir)?;
    SocialPlatform::export_all_to(out_dir)?;
    SocialLink::export_all_to(out_dir)?;
    EquipmentSummary::export_all_to(out_dir)?;
    LocationSummary::export_all_to(out_dir)?;
    Profile::export_all_to(out_dir)?;
    HeroStats::export_all_to(out_dir)?;
    FeaturedPhotoSummary::export_all_to(out_dir)?;
    PublicProfile::export_all_to(out_dir)?;
    GalleryPhoto::export_all_to(out_dir)?;
    GalleryPage::export_all_to(out_dir)?;
    DiscoveryPhoto::export_all_to(out_dir)?;
    DiscoveryPage::export_all_to(out_dir)?;
    TargetMeta::export_all_to(out_dir)?;
    TargetPage::export_all_to(out_dir)?;
    TagMeta::export_all_to(out_dir)?;
    TagPage::export_all_to(out_dir)?;
    EquipmentMeta::export_all_to(out_dir)?;
    EquipmentPaired::export_all_to(out_dir)?;
    EquipmentPage::export_all_to(out_dir)?;
    CategoryPage::export_all_to(out_dir)?;
    SearchTargetHit::export_all_to(out_dir)?;
    SearchUserHit::export_all_to(out_dir)?;
    SearchResults::export_all_to(out_dir)?;
    DraftListItem::export_all_to(out_dir)?;
    DraftListResponse::export_all_to(out_dir)?;
    BatchApplyRequest::export_all_to(out_dir)?;
    BatchApplyResponse::export_all_to(out_dir)?;
    BatchPublishRequest::export_all_to(out_dir)?;
    PublishedItem::export_all_to(out_dir)?;
    SkipReason::export_all_to(out_dir)?;
    SkippedItem::export_all_to(out_dir)?;
    BatchPublishResponse::export_all_to(out_dir)?;
    PatchTargetsItem::export_all_to(out_dir)?;
    PatchTargetsResponse::export_all_to(out_dir)?;
    TargetPreviewThumb::export_all_to(out_dir)?;
    TargetListItem::export_all_to(out_dir)?;
    TargetIndexPage::export_all_to(out_dir)?;
    StorageSummary::export_all_to(out_dir)?;
    PhotographerListItem::export_all_to(out_dir)?;
    PhotographerIndexPage::export_all_to(out_dir)?;

    println!("Wrote types to: {}", out_dir.display());
    Ok(())
}
