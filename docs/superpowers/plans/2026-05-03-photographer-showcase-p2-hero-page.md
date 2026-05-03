# Photographer Showcase — Phase 2 Hero Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild `/u/<handle>` into a polished public profile that doubles as a portfolio: cover banner, identity, sanitised rich-text bio, equipment strip, location/sky badge, stats row, 6-slot featured row, justified-rows gallery, and a deep-linked lightbox. Add the profile editor (modal, save-on-blur per section), cover picker, and drag-reorderable featured controls. The schema and the core read endpoints already shipped in P1; P2 wires them into a complete UI surface and adds the remaining write endpoints (cover, featured pin/unpin/reorder) and the public-profile aggregator.

**Architecture:** Backend gets four new endpoints — `POST /api/me/cover`, `POST/DELETE /api/me/featured/:photo_id`, `PATCH /api/me/featured/order`, plus a public aggregator `GET /api/users/by-handle/:handle/profile` that returns everything the hero needs in one round-trip — and an extended `PATCH /api/me/profile` that runs every `bio_html` write through `users::bio::sanitize` (ammonia) regardless of the client. The frontend rebuilds `frontend/src/routes/u/[handle]/+page.svelte` around a 13-component hero shell, a Tiptap-backed profile editor whose mark/node whitelist mirrors the ammonia allowlist exactly, a cover picker, drag-reorderable featured row (`@neodrag/svelte`), a justified-rows gallery (`justified-layout` — already added in P1), and a lightbox at `/u/[handle]/p/[short_id]` that mounts via SvelteKit **shallow routing** (`pushState` + `preloadData`) when navigated from the gallery and as a full photo-detail page on direct visit. `social_links` is a typed `Vec<{platform, url}>` validated server-side (URL parse + platform whitelist) before persisting to the existing jsonb column. Featured pin/unpin/reorder writes are wrapped in a single transaction with the partial-unique constraint on `(owner_id, featured_position) WHERE featured_at IS NOT NULL` (migration 0009) staged via NULL-then-target updates so the index never trips mid-update.

**Tech Stack:** Rust 2024 + axum 0.7 + sqlx 0.8 (compile-time-checked queries) + ammonia 4 (shipped P1) + Postgres 16; SvelteKit 2 + Svelte 5 runes + `@tiptap/core` 2 + `@tiptap/pm` 2 + `@tiptap/starter-kit` 2 + `@neodrag/svelte` 2 + `justified-layout` 4 (shipped P1) + ts-rs for Rust→TS types. No new migrations are required — P1 already shipped 0008 (`tagline`, `bio_html`, `cover_photo_id`, `equipment_*`, `location_text`, `bortle_class`, `sqm`, `social_links`), 0009 (`featured_at`, `featured_position`), and 0011 (`appreciations_count`).

**Spec:** `docs/superpowers/specs/2026-05-03-photographer-showcase-design.md` — read the P2 section (lines 542–789) before starting.

**Design handoff (canonical for layout, dimensions, copy):** `/Users/pleclech/Downloads/design_handoff_astrophoto 3/showcase/showcase-p2.jsx` and the shared `styles.css` next to it. The plan's task code shows behaviour (props, state, handlers, server queries, tests) and the structural Svelte markup; exact pixel values, colour token usage, and copy come from the handoff. Where this plan and the handoff disagree on copy, the handoff wins.

**Dual-canonical rule (inherited from P1):** the spec is canonical for behaviour, schema, URLs, and security; the design files are canonical for layout, hierarchy, and copy.

---

## Branch and worktree

The worktree is already created at `/Volumes/Pascal4Tb/Projects/astrophoto-showcase-p2` on branch `feat/showcase-p2-hero` (off `main` at `e8aa4e8`). All commits land on this branch. Run every command in this plan from the worktree root unless a step says otherwise.

After the final acceptance task: merge to `main` via `gh pr merge <num> --merge`. The repo has `delete_branch_on_merge: false`, so manual cleanup (`git worktree remove`, `git branch -d`, `git push origin --delete`) is the operator's responsibility — see the **After merge** section at the bottom of this plan.

**No Playwright for P2 acceptance.** Per the project memory `E2E tooling — chrome-devtools-mcp not Playwright`, end-to-end acceptance for P2 is a `chrome-devtools-mcp` browser walk recorded in `docs/operations/p2-acceptance.md`. P1 left an unintended `frontend/tests/e2e/p1-happy-path.spec.ts` in the tree; P2 must not add to that file or create a sibling spec.

---

## File structure (where things land)

**Backend, new files:**

- `backend/src/users/cover.rs` — `POST /api/me/cover` handler. Validates the supplied `photo_id` is owned, published, and `status='ready'` before writing `users.cover_photo_id`.
- `backend/src/users/featured.rs` — `POST /api/me/featured/:photo_id`, `DELETE /api/me/featured/:photo_id`, `PATCH /api/me/featured/order`. All three share a single transactional helper for safety against the partial-unique index.
- `backend/src/users/public_profile.rs` — `GET /api/users/by-handle/:handle/profile`. Read-side aggregator returning the `PublicProfile` wire type (everything the hero shell needs in one query — joins users, photos counts, stats, equipment).
- `backend/src/users/photos_feed.rs` — `GET /api/users/by-handle/:handle/photos`. Cursor-paginated published-photos feed, sortable.
- `backend/src/users/social_links.rs` — wire type + URL/platform validation used by `profile.rs` PATCH.
- `backend/tests/profile_extended.rs` — extended PATCH coverage (every field, sanitiser invocation, social_links rejection cases).
- `backend/tests/cover_set.rs`
- `backend/tests/featured_pin.rs`
- `backend/tests/featured_reorder.rs`
- `backend/tests/public_profile.rs`
- `backend/tests/photos_feed.rs`

**Backend, modified files:**

- `backend/src/api_types.rs` — extend `Profile` and add `PublicProfile`, `SocialLink`, `SocialPlatform`, `EquipmentSummary`, `LocationSummary`, `HeroStats`, `FeaturedPhotoSummary`, `GalleryPhoto`, `GalleryPage`.
- `backend/src/users/mod.rs` — register the new handlers.
- `backend/src/users/profile.rs` — extend GET to return the full profile, extend PATCH to write every field with bio sanitisation and social-links validation.
- `backend/src/http/mod.rs` — register the new routes.

**Frontend, new files (under `frontend/src/lib/components/profile/`):**

- `HeroPage.svelte` — orchestrates layout for the three view modes (`visitor`, `owner`, `admin`).
- `OwnerModeBanner.svelte`
- `HeroCover.svelte`
- `HeroIdentity.svelte` — three-column grid wrapper.
- `HeroAvatar.svelte`
- `HeroName.svelte`
- `HeroTagline.svelte`
- `HeroSocialLinks.svelte`
- `HeroActions.svelte`
- `HeroAbout.svelte`
- `HeroEquipmentStrip.svelte`
- `HeroLocationBadge.svelte`
- `HeroStatsRow.svelte`
- `FeaturedRow.svelte`
- `FeaturedTile.svelte`
- `GalleryToolbar.svelte`
- `PhotoGrid.svelte` — wraps `justified-layout`.
- `PhotoTile.svelte` — gallery tile (different from `FeaturedTile`).

**Frontend, profile editor (under `frontend/src/lib/components/profile/editor/`):**

- `ProfileEditor.svelte` — modal shell, sectioned save-on-blur.
- `IdentitySection.svelte`
- `AboutSection.svelte` — Tiptap editor.
- `tiptapAllowlist.ts` — exports the mark/node whitelist constants used both to configure Tiptap and to assert (in a unit test) that they match `users::bio::sanitize`.
- `EquipmentSection.svelte` — five autocomplete cells.
- `LocationSection.svelte`
- `BortleLadder.svelte` — 9-cell segmented control.
- `SocialLinksSection.svelte` — repeater, platform picker.
- `CoverPickerModal.svelte`
- `PhotoPickerGrid.svelte` — shared between cover picker and featured pin (multiselect/single via prop).

**Frontend, lightbox (under `frontend/src/lib/components/lightbox/`):**

- `Lightbox.svelte` — two-column layout (full-bleed image left, EXIF panel right), keyboard handling.
- `LightboxExifPanel.svelte`
- `MoreFromPhotographerStrip.svelte`

**Frontend, shared utilities:**

- `frontend/src/lib/format/integration.ts` — pretty-prints integration time as `86h 14m`.
- `frontend/src/lib/format/cdnUrl.ts` — extend if not already present (P1 added a basic builder; verify in Task 4).
- `frontend/src/lib/api/profile.ts` — small client wrappers around the new endpoints (`fetchPublicProfile`, `setCover`, `pinFeatured`, `unpinFeatured`, `reorderFeatured`, `fetchPhotosFeed`).

**Frontend, route files:**

- `frontend/src/routes/u/[handle]/+page.server.ts` — rewritten to call the new aggregator endpoint.
- `frontend/src/routes/u/[handle]/+page.svelte` — replaced by `<HeroPage>` orchestration.
- `frontend/src/routes/u/[handle]/p/[short_id]/+page.server.ts` — extended to also load the photographer's neighbouring photos (for `<MoreFromPhotographerStrip>`) and the prev/next short_ids.
- `frontend/src/routes/u/[handle]/p/[short_id]/+page.svelte` — when entered via shallow routing (state has `lightbox: true`), renders inside `<Lightbox>` overlay; on direct visit, renders the full photo-detail page that already exists.

**Docs:**

- `docs/operations/p2-acceptance.md` — created at the end with the chrome-devtools-mcp acceptance walk.

---

## Setup

### Task 1: Add frontend deps — Tiptap and @neodrag/svelte

**Files:**
- Modify: `frontend/package.json`, `frontend/pnpm-lock.yaml`

- [ ] **Step 1: Add the runtime deps**

Run from the worktree root:

```
cd frontend && pnpm add @tiptap/core @tiptap/pm @tiptap/starter-kit @tiptap/extension-link @neodrag/svelte
```

Notes on the picks:
- `@tiptap/starter-kit` already includes `paragraph`, `heading`, `bold`, `italic`, `bulletList`, `orderedList`, `listItem`, `blockquote`, `hardBreak`, `history`, `dropcursor`, `gapcursor`. We'll restrict it to the ammonia allowlist via Tiptap config in Task 38.
- `@tiptap/extension-link` is separate from starter-kit; we need it for the bio link button.
- `@neodrag/svelte@2.x` ships first-class Svelte 5 support.

- [ ] **Step 2: Verify svelte-check is still clean**

```
cd frontend && pnpm install && pnpm check
```

Expected: `pnpm check` exits 0 (no svelte-check errors). If it complains about a missing peer for `@tiptap/pm` (ProseMirror collab modules), accept the warning — it's optional and we don't enable collab.

- [ ] **Step 3: Commit**

```
git add frontend/package.json frontend/pnpm-lock.yaml
git commit -m "chore(frontend): add tiptap + neodrag for showcase P2"
```

---

### Task 2: Confirm shipped P2 schema and existing scaffolding

This is a read-only orientation task — no commit. Confirm the migration list and key handlers exist exactly as P1 left them. If anything below is missing, stop and surface it before continuing; the rest of the plan assumes these.

- [ ] **Step 1: Verify migrations 0008, 0009, 0011 are present and contain the expected columns**

```
ls backend/migrations/0008_user_profile.sql backend/migrations/0009_photo_featured_category.sql backend/migrations/0011_appreciations_count.sql
grep -E "tagline|bio_html|cover_photo_id|social_links|equipment_telescope|equipment_camera|equipment_mount|equipment_filters|equipment_guiding|location_text|bortle_class|sqm" backend/migrations/0008_user_profile.sql
grep -E "featured_at|featured_position|photos_featured_per_user_uidx|photos_featured_pair_chk|category" backend/migrations/0009_photo_featured_category.sql
grep -E "appreciations_count" backend/migrations/0011_appreciations_count.sql
```

Expected: every grep prints at least one matching line.

- [ ] **Step 2: Verify ammonia bio sanitiser exists and inspect its allowlist**

```
test -f backend/src/users/bio.rs
grep -nE "fn sanitize|tags|tag_attributes" backend/src/users/bio.rs
```

Expected: `backend/src/users/bio.rs` exists and exports `sanitize`. The current allowlist is exactly `p, br, strong, em, u, h2, h3, h4, ul, ol, li, blockquote, code, a` (with `href` only on `<a>`, `http`/`https`/`mailto` schemes, `rel="nofollow noopener"` forced). Tasks 5, 36, and 37 all reference this list verbatim — if it differs from the above, stop and surface the divergence before continuing.

- [ ] **Step 3: Verify the existing `/api/me/profile` and `/api/users/by-handle/:handle` routes**

```
grep -nE "/api/me/profile|/api/users/by-handle" backend/src/http/mod.rs
```

Expected: both routes are registered. Do not commit — this task is verification only.

---

### Task 3: Extract the bio allowlist into a shared JSON source of truth

The Rust ammonia builder and the Tiptap editor must agree on the exact tag set; drift is either silent stripping at save or an XSS hole. Centralise the list as JSON, read it from both sides, and add tests that fail on drift.

**Files:**
- Create: `backend/data/bio-allowed-tags.json`
- Modify: `backend/src/users/bio.rs`
- Test: existing `backend/src/users/bio.rs` `mod tests` block — add one new case.

- [ ] **Step 1: Write the failing test in `backend/src/users/bio.rs`**

Append inside the existing `#[cfg(test)] mod tests { ... }`:

```rust
    #[test]
    fn allowlist_matches_shared_json() {
        // Source of truth shared with the frontend Tiptap config.
        let raw = include_str!("../../data/bio-allowed-tags.json");
        let json: serde_json::Value = serde_json::from_str(raw)
            .expect("bio-allowed-tags.json must be valid JSON");
        let arr = json
            .get("tags")
            .and_then(|v| v.as_array())
            .expect("bio-allowed-tags.json must have a top-level `tags` array");
        let from_json: std::collections::BTreeSet<String> = arr
            .iter()
            .map(|v| v.as_str().expect("tag must be string").to_owned())
            .collect();

        let from_code: std::collections::BTreeSet<String> = ALLOWED_TAGS
            .iter()
            .map(|s| (*s).to_owned())
            .collect();

        assert_eq!(
            from_json, from_code,
            "bio.rs ALLOWED_TAGS and bio-allowed-tags.json have drifted"
        );
    }
```

- [ ] **Step 2: Run the test — it must fail because `ALLOWED_TAGS` and the JSON file don't exist yet**

```
cd backend && cargo test -p astrophoto users::bio::tests::allowlist_matches_shared_json
```

Expected: compilation error (`cannot find value 'ALLOWED_TAGS' in this scope`) or `include_str!` macro error.

- [ ] **Step 3: Create `backend/data/bio-allowed-tags.json`**

```json
{
  "tags": [
    "a",
    "blockquote",
    "br",
    "code",
    "em",
    "h2",
    "h3",
    "h4",
    "li",
    "ol",
    "p",
    "strong",
    "u",
    "ul"
  ],
  "anchorAttributes": ["href"],
  "anchorSchemes": ["http", "https", "mailto"],
  "anchorRel": "nofollow noopener"
}
```

- [ ] **Step 4: Refactor `backend/src/users/bio.rs` to read from the shared constant**

Replace the entire file body with:

```rust
//! Bio HTML sanitisation. Server is the source of truth — any HTML
//! posted to PATCH /api/me/profile passes through `sanitize`. The Tiptap
//! client editor (P2) is configured to emit only this same allowlist;
//! the JSON file at `backend/data/bio-allowed-tags.json` is the shared
//! source of truth between Rust and TypeScript.

use ammonia::Builder;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

pub const ALLOWED_TAGS: &[&str] = &[
    "a", "blockquote", "br", "code", "em", "h2", "h3", "h4", "li", "ol", "p",
    "strong", "u", "ul",
];

const ANCHOR_SCHEMES: &[&str] = &["http", "https", "mailto"];
const ANCHOR_REL: &str = "nofollow noopener";

pub fn sanitize(input: &str) -> String {
    cleaner().clean(input).to_string()
}

fn cleaner() -> &'static Builder<'static> {
    static C: OnceLock<Builder<'static>> = OnceLock::new();
    C.get_or_init(|| {
        let mut b = Builder::default();
        b.tags(ALLOWED_TAGS.iter().copied().collect::<HashSet<_>>());
        b.tag_attributes(HashMap::from([("a", HashSet::from(["href"]))]));
        b.url_schemes(ANCHOR_SCHEMES.iter().copied().collect::<HashSet<_>>());
        b.link_rel(Some(ANCHOR_REL));
        b
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_safe_tags() {
        let out = sanitize("<p>Hello <strong>world</strong></p>");
        assert!(out.contains("<strong>"));
    }

    #[test]
    fn strips_script() {
        let out = sanitize("<p>Hi</p><script>alert('x')</script>");
        assert!(!out.contains("script"));
    }

    #[test]
    fn strips_onclick() {
        let out = sanitize("<a href=\"https://x\" onclick=\"x()\">l</a>");
        assert!(!out.contains("onclick"));
    }

    #[test]
    fn strips_javascript_uri() {
        let out = sanitize("<a href=\"javascript:alert(1)\">l</a>");
        assert!(!out.contains("javascript:"));
    }

    #[test]
    fn forces_rel_on_links() {
        let out = sanitize("<a href=\"https://x\">l</a>");
        assert!(out.contains("rel=\"nofollow noopener\""));
    }

    #[test]
    fn strips_iframe() {
        let out = sanitize("<iframe src=\"https://x\"></iframe>");
        assert!(!out.contains("iframe"));
    }

    #[test]
    fn keeps_lists() {
        let out = sanitize("<ul><li>a</li><li>b</li></ul>");
        assert!(out.contains("<ul>") && out.contains("<li>"));
    }

    #[test]
    fn allowlist_matches_shared_json() {
        let raw = include_str!("../../data/bio-allowed-tags.json");
        let json: serde_json::Value = serde_json::from_str(raw)
            .expect("bio-allowed-tags.json must be valid JSON");
        let arr = json
            .get("tags")
            .and_then(|v| v.as_array())
            .expect("bio-allowed-tags.json must have a top-level `tags` array");
        let from_json: std::collections::BTreeSet<String> = arr
            .iter()
            .map(|v| v.as_str().expect("tag must be string").to_owned())
            .collect();
        let from_code: std::collections::BTreeSet<String> =
            ALLOWED_TAGS.iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(
            from_json, from_code,
            "bio.rs ALLOWED_TAGS and bio-allowed-tags.json have drifted"
        );
    }
}
```

- [ ] **Step 5: Run all bio tests**

```
cd backend && cargo test -p astrophoto users::bio
```

Expected: all 8 tests pass.

- [ ] **Step 6: Commit**

```
git add backend/data/bio-allowed-tags.json backend/src/users/bio.rs
git commit -m "refactor(backend): bio allowlist as shared JSON source of truth

The same list is consumed by P2's Tiptap editor on the frontend; this
test catches drift before it ships."
```

---

### Task 4: Define the wire types — Profile, PublicProfile, SocialLink, etc.

Add or extend the ts-rs-derived types that frontends rely on. After this task, `frontend/src/lib/api/Profile.ts` will be regenerated and grow several new types.

**Files:**
- Modify: `backend/src/api_types.rs`
- Generated: `frontend/src/lib/api/Profile.ts` (re-run `just types` in Step 5)

- [ ] **Step 1: Read the current `backend/src/api_types.rs`**

```
grep -n "Profile" backend/src/api_types.rs
```

Note where `Profile` is currently declared. Keep the existing struct intact; we extend it.

- [ ] **Step 2: Replace the `Profile` definition with the extended set**

In `backend/src/api_types.rs`, replace the existing `Profile` struct with the block below. Add the new types in the same file in the order shown.

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SocialLink {
    pub platform: SocialPlatform,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EquipmentSummary {
    pub telescope: Option<String>,
    pub camera: Option<String>,
    pub mount: Option<String>,
    pub filters: Option<String>,
    pub guiding: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LocationSummary {
    pub location_text: Option<String>,
    pub bortle_class: Option<i16>,
    pub sqm: Option<f64>,
}

/// Authenticated owner's view of their own profile (writable surface).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Profile {
    pub display_name: String,
    pub tagline: Option<String>,
    pub bio_html: Option<String>,
    pub cover_photo_id: Option<Uuid>,
    pub equipment: EquipmentSummary,
    pub location: LocationSummary,
    pub social_links: Vec<SocialLink>,
}

/// Patch body — every field is optional; absent = leave alone.
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

/// Helper: distinguishes "field absent" from "field present and null".
fn deserialize_some<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HeroStats {
    pub frames: i64,
    pub integration_seconds: i64,
    pub followers: i64,
    pub appreciations: i64,
    pub targets: i64,
    pub member_since_year: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
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
#[ts(export)]
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
#[ts(export)]
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
#[ts(export)]
pub struct GalleryPage {
    pub photos: Vec<GalleryPhoto>,
    /// Opaque cursor; pass back as `?cursor=` to load the next page. `None` when exhausted.
    pub next_cursor: Option<String>,
}
```

- [ ] **Step 3: Verify it compiles**

```
cd backend && cargo check
```

Expected: clean build. If `chrono` or `uuid` aren't already in `Cargo.toml`, they are — they're used elsewhere in the crate.

- [ ] **Step 4: Re-export from the right path if needed**

```
grep -nE "pub use api_types::" backend/src/lib.rs backend/src/main.rs 2>/dev/null
```

If `api_types` items are re-exported (the existing `Profile` was), add the new types to that re-export list. Otherwise leave it.

- [ ] **Step 5: Regenerate the TS types**

```
just types
```

Expected: `frontend/src/lib/api/Profile.ts` and several new files appear under `frontend/src/lib/api/` (one per `#[ts(export)]` type, by ts-rs convention). Run `cd frontend && pnpm check` and verify no svelte-check errors. If existing call sites break (the old `Profile` had only `display_name`), that is expected — Tasks 6 and 7 fix the consumers; commit the type change first and don't suppress the errors.

- [ ] **Step 6: Commit**

```
git add backend/src/api_types.rs frontend/src/lib/api/
git commit -m "feat(api): extended Profile + PublicProfile + supporting types

Schema for the P2 hero-page wire format. Tiptap-produced bio_html is
sanitised server-side regardless of client; social_links are validated
against a platform whitelist (Task 5). Existing /api/me/profile callers
break here; Tasks 6 and 7 fix them."
```

---

### Task 5: Server-side validation for SocialLink

`social_links` is jsonb with no DB constraint. Add a typed validator that platform-and-URL checks every entry before persistence.

**Files:**
- Create: `backend/src/users/social_links.rs`
- Modify: `backend/src/users/mod.rs`
- Test: `backend/src/users/social_links.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Write the failing tests**

Create `backend/src/users/social_links.rs` with:

```rust
//! Validation for the `social_links` jsonb column. The DB doesn't
//! constrain shape; this module is the only legitimate writer.

use crate::AppError;
use crate::api_types::{SocialLink, SocialPlatform};
use url::Url;

const MAX_LINKS: usize = 6;

pub fn validate_links(links: &[SocialLink]) -> Result<(), AppError> {
    if links.len() > MAX_LINKS {
        return Err(AppError::bad_request("social_links_too_many"));
    }
    let mut seen = std::collections::HashSet::new();
    for link in links {
        if !seen.insert(link.platform.clone()) {
            return Err(AppError::bad_request("social_links_duplicate_platform"));
        }
        validate_one(link)?;
    }
    Ok(())
}

fn validate_one(link: &SocialLink) -> Result<(), AppError> {
    let parsed = Url::parse(&link.url).map_err(|_| AppError::bad_request("social_link_url_invalid"))?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(AppError::bad_request("social_link_url_scheme"));
    }
    let host = parsed.host_str().ok_or_else(|| AppError::bad_request("social_link_url_no_host"))?;
    let host = host.to_ascii_lowercase();
    let allowed = match link.platform {
        SocialPlatform::Twitter   => &["twitter.com", "x.com"][..],
        SocialPlatform::Instagram => &["instagram.com", "www.instagram.com"][..],
        SocialPlatform::Bluesky   => &["bsky.app"][..],
        SocialPlatform::Astrobin  => &["astrobin.com", "www.astrobin.com"][..],
        SocialPlatform::Mastodon  => &[][..], // any host — many instances
        SocialPlatform::Youtube   => &["youtube.com", "www.youtube.com", "youtu.be"][..],
        SocialPlatform::Website   => &[][..], // any host
    };
    if !allowed.is_empty() && !allowed.iter().any(|d| host == *d || host.ends_with(&format!(".{d}"))) {
        return Err(AppError::bad_request("social_link_host_mismatch"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sl(p: SocialPlatform, url: &str) -> SocialLink {
        SocialLink { platform: p, url: url.into() }
    }

    #[test]
    fn accepts_canonical_twitter() {
        validate_links(&[sl(SocialPlatform::Twitter, "https://twitter.com/marie")]).unwrap();
    }

    #[test]
    fn accepts_x_com_for_twitter() {
        validate_links(&[sl(SocialPlatform::Twitter, "https://x.com/marie")]).unwrap();
    }

    #[test]
    fn rejects_wrong_host_for_platform() {
        let err = validate_links(&[sl(SocialPlatform::Twitter, "https://evil.example/marie")]).unwrap_err();
        assert_eq!(format!("{err:?}").contains("social_link_host_mismatch"), true);
    }

    #[test]
    fn rejects_javascript_scheme() {
        let err = validate_links(&[sl(SocialPlatform::Website, "javascript:alert(1)")]).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("social_link_url_scheme") || msg.contains("social_link_url_invalid"));
    }

    #[test]
    fn rejects_more_than_six() {
        let many = std::iter::repeat(sl(SocialPlatform::Website, "https://a.example"))
            .take(7)
            .collect::<Vec<_>>();
        let err = validate_links(&many).unwrap_err();
        assert!(format!("{err:?}").contains("social_links_too_many"));
    }

    #[test]
    fn rejects_duplicate_platform() {
        let links = vec![
            sl(SocialPlatform::Twitter, "https://twitter.com/a"),
            sl(SocialPlatform::Twitter, "https://x.com/b"),
        ];
        let err = validate_links(&links).unwrap_err();
        assert!(format!("{err:?}").contains("social_links_duplicate_platform"));
    }

    #[test]
    fn website_accepts_any_https_host() {
        validate_links(&[sl(SocialPlatform::Website, "https://marie.example.com/")]).unwrap();
    }

    #[test]
    fn mastodon_accepts_any_https_host() {
        validate_links(&[sl(SocialPlatform::Mastodon, "https://mastodon.social/@marie")]).unwrap();
    }
}
```

- [ ] **Step 2: Add `pub mod social_links;` to `backend/src/users/mod.rs`**

```
grep -n "pub mod" backend/src/users/mod.rs
```

Append `pub mod social_links;` next to the other `pub mod ...` lines (alphabetical-adjacent). Do NOT remove any existing module declarations.

- [ ] **Step 3: Add the `url` crate if not already present**

```
grep -E "^url\s*=" backend/Cargo.toml
```

If absent:

```
cd backend && cargo add url@2
```

- [ ] **Step 4: Run the tests**

```
cd backend && cargo test -p astrophoto users::social_links
```

Expected: all 8 tests pass.

- [ ] **Step 5: Commit**

```
git add backend/src/users/social_links.rs backend/src/users/mod.rs backend/Cargo.toml backend/Cargo.lock
git commit -m "feat(users): typed validator for social_links jsonb"
```

---

### Task 6: Extend GET /api/me/profile to return the full owner view

The existing handler returns only `display_name`. Replace it to read all P2 fields and return the new `Profile` shape from Task 4.

**Files:**
- Modify: `backend/src/users/profile.rs`
- Test: `backend/tests/profile_extended.rs` (created in this task; appended to in Task 7)

- [ ] **Step 1: Write the failing test**

Create `backend/tests/profile_extended.rs`:

```rust
mod common;

use astrophoto::api_types::{Profile, SocialLink, SocialPlatform};
use common::TestApp;

#[tokio::test]
async fn get_profile_returns_full_shape_for_fresh_user() {
    let app = TestApp::launch().await;
    let (cookie, _) = app.signup_with_handle("Marie", "marie", "marie@x.test").await;

    let res = app
        .client
        .get(format!("{}/api/me/profile", app.base_url))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    let body: Profile = res.json().await.unwrap();
    assert_eq!(body.display_name, "Marie");
    assert!(body.tagline.is_none());
    assert!(body.bio_html.is_none());
    assert!(body.cover_photo_id.is_none());
    assert!(body.equipment.telescope.is_none());
    assert!(body.equipment.camera.is_none());
    assert!(body.equipment.mount.is_none());
    assert!(body.equipment.filters.is_none());
    assert!(body.equipment.guiding.is_none());
    assert!(body.location.location_text.is_none());
    assert!(body.location.bortle_class.is_none());
    assert!(body.location.sqm.is_none());
    assert_eq!(body.social_links, Vec::<SocialLink>::new());
    let _ = SocialPlatform::Website; // import-touch
}
```

(`common::TestApp` and `signup_with_handle` already exist from P1's test harness — verify with `grep -n "signup_with_handle" backend/tests/common.rs`. If signature differs, adapt the call here to match what's already there.)

- [ ] **Step 2: Run the test — fails because GET still returns the old shape**

```
cd backend && cargo test -p astrophoto --test profile_extended get_profile_returns_full_shape_for_fresh_user -- --nocapture
```

Expected: deserialisation failure (the old handler returns `{display_name}` only, so missing keys won't deserialise into the extended `Profile`).

- [ ] **Step 3: Replace the GET handler in `backend/src/users/profile.rs`**

Open `backend/src/users/profile.rs`. Replace the existing `get` function with:

```rust
pub async fn get(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Profile>, AppError> {
    let row = sqlx::query!(
        r#"
        select
            display_name,
            tagline,
            bio_html,
            cover_photo_id,
            equipment_telescope,
            equipment_camera,
            equipment_mount,
            equipment_filters,
            equipment_guiding,
            location_text,
            bortle_class,
            sqm,
            social_links
        from users
        where id = $1
        "#,
        user.id
    )
    .fetch_one(&state.pool)
    .await?;

    let social_links: Vec<SocialLink> = serde_json::from_value(row.social_links)
        .map_err(|_| AppError::internal("social_links_corrupt"))?;

    Ok(Json(Profile {
        display_name: row.display_name,
        tagline: row.tagline,
        bio_html: row.bio_html,
        cover_photo_id: row.cover_photo_id,
        equipment: EquipmentSummary {
            telescope: row.equipment_telescope,
            camera: row.equipment_camera,
            mount: row.equipment_mount,
            filters: row.equipment_filters,
            guiding: row.equipment_guiding,
        },
        location: LocationSummary {
            location_text: row.location_text,
            bortle_class: row.bortle_class,
            sqm: row.sqm.map(|d: sqlx::types::Decimal| d.to_string().parse::<f64>().unwrap_or(0.0)),
        },
        social_links,
    }))
}
```

Update the imports at the top of the file accordingly:

```rust
use crate::api_types::{EquipmentSummary, LocationSummary, Profile, SocialLink};
```

(The `Decimal` → `f64` conversion is intentionally lossy with a default of `0.0` on parse failure; SQM values fit in `numeric(4,2)` — at most 99.99 — so float precision is not a concern. If `sqlx::types::Decimal` is not the actual type sqlx infers for `numeric(4,2)`, run `cargo check` and adjust to whatever the compiler says.)

- [ ] **Step 4: Re-run sqlx prepare**

```
cd backend && cargo sqlx prepare -- --tests
```

Expected: `.sqlx/` updated. Stage the diff in Step 8.

- [ ] **Step 5: Run the test**

```
cd backend && cargo test -p astrophoto --test profile_extended get_profile_returns_full_shape_for_fresh_user
```

Expected: PASS.

- [ ] **Step 6: Run the full backend test suite to catch regressions**

```
cd backend && cargo test --tests
```

Expected: every suite that reads `Profile` shape compiles. If a Phase 8a test (e.g. `tests/auth_me`) breaks because it asserts the old shape, update its assertions to match the new shape — but only the assertions, not behaviour.

- [ ] **Step 7: Commit**

```
git add backend/src/users/profile.rs backend/tests/profile_extended.rs backend/.sqlx/
git commit -m "feat(profile): GET /api/me/profile returns full P2 owner view"
```

---

### Task 7: Extend PATCH /api/me/profile to write every field with sanitisation

`bio_html` MUST go through `users::bio::sanitize`; `social_links` MUST go through `users::social_links::validate_links`. The patch shape uses double-`Option` so absent fields are left alone but explicit `null` clears.

**Files:**
- Modify: `backend/src/users/profile.rs`
- Test: append to `backend/tests/profile_extended.rs`

- [ ] **Step 1: Write the failing tests — append to `backend/tests/profile_extended.rs`**

```rust
#[tokio::test]
async fn patch_profile_writes_tagline_and_bio_with_sanitisation() {
    let app = TestApp::launch().await;
    let (cookie, _) = app.signup_with_handle("Marie", "marie", "marie@x.test").await;

    let body = serde_json::json!({
        "tagline": "Hunting deep-sky from a Bortle 6 backyard",
        "bio_html": "<p>Hi <strong>world</strong></p><script>alert(1)</script>"
    });

    let res = app
        .client
        .patch(format!("{}/api/me/profile", app.base_url))
        .header("cookie", cookie.clone())
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204);

    // Read back via GET.
    let res = app
        .client
        .get(format!("{}/api/me/profile", app.base_url))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    let body: Profile = res.json().await.unwrap();
    assert_eq!(body.tagline.as_deref(), Some("Hunting deep-sky from a Bortle 6 backyard"));
    let bio = body.bio_html.unwrap();
    assert!(bio.contains("<strong>"));
    assert!(!bio.contains("<script>"), "script must be stripped: {bio}");
}

#[tokio::test]
async fn patch_profile_writes_equipment_and_location() {
    let app = TestApp::launch().await;
    let (cookie, _) = app.signup_with_handle("Marie", "marie", "marie@x.test").await;

    let body = serde_json::json!({
        "equipment": {
            "telescope": "RedCat 51",
            "camera": "ZWO ASI2600MC",
            "mount": "ZWO AM5",
            "filters": "Optolong L-Pro",
            "guiding": "ASI120MM"
        },
        "location": {
            "location_text": "Lyon, FR",
            "bortle_class": 6,
            "sqm": 19.8
        }
    });

    let res = app
        .client
        .patch(format!("{}/api/me/profile", app.base_url))
        .header("cookie", cookie.clone())
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204);

    let res = app
        .client
        .get(format!("{}/api/me/profile", app.base_url))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    let body: Profile = res.json().await.unwrap();
    assert_eq!(body.equipment.telescope.as_deref(), Some("RedCat 51"));
    assert_eq!(body.equipment.camera.as_deref(), Some("ZWO ASI2600MC"));
    assert_eq!(body.location.location_text.as_deref(), Some("Lyon, FR"));
    assert_eq!(body.location.bortle_class, Some(6));
    assert!((body.location.sqm.unwrap() - 19.8).abs() < 0.01);
}

#[tokio::test]
async fn patch_profile_validates_social_links() {
    let app = TestApp::launch().await;
    let (cookie, _) = app.signup_with_handle("Marie", "marie", "marie@x.test").await;

    // Wrong host for the named platform → 400.
    let bad = serde_json::json!({
        "social_links": [{ "platform": "twitter", "url": "https://evil.example/marie" }]
    });
    let res = app
        .client
        .patch(format!("{}/api/me/profile", app.base_url))
        .header("cookie", cookie.clone())
        .json(&bad)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);

    // Correct mapping → 204 + persisted.
    let ok = serde_json::json!({
        "social_links": [
            { "platform": "twitter",   "url": "https://twitter.com/marie" },
            { "platform": "instagram", "url": "https://instagram.com/marie" }
        ]
    });
    let res = app
        .client
        .patch(format!("{}/api/me/profile", app.base_url))
        .header("cookie", cookie.clone())
        .json(&ok)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204);

    let res = app
        .client
        .get(format!("{}/api/me/profile", app.base_url))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    let body: Profile = res.json().await.unwrap();
    assert_eq!(body.social_links.len(), 2);
}

#[tokio::test]
async fn patch_profile_clears_field_when_explicit_null() {
    let app = TestApp::launch().await;
    let (cookie, _) = app.signup_with_handle("Marie", "marie", "marie@x.test").await;

    let set = serde_json::json!({ "tagline": "first" });
    app.client
        .patch(format!("{}/api/me/profile", app.base_url))
        .header("cookie", cookie.clone())
        .json(&set)
        .send()
        .await
        .unwrap();

    let clear = serde_json::json!({ "tagline": null });
    let res = app
        .client
        .patch(format!("{}/api/me/profile", app.base_url))
        .header("cookie", cookie.clone())
        .json(&clear)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204);

    let res = app
        .client
        .get(format!("{}/api/me/profile", app.base_url))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    let body: Profile = res.json().await.unwrap();
    assert!(body.tagline.is_none());
}
```

- [ ] **Step 2: Run the tests — they must fail because PATCH is still display-name-only**

```
cd backend && cargo test -p astrophoto --test profile_extended -- --nocapture
```

Expected: the four new tests fail; the GET test from Task 6 still passes.

- [ ] **Step 3: Replace the PATCH handler in `backend/src/users/profile.rs`**

Replace the entire `PutBody` and `put` items with the block below. (Yes, the route name stays `put` for module-naming continuity even though the HTTP verb is PATCH; the routing wires `.patch(...)` to this fn.)

```rust
use crate::api_types::{
    EquipmentSummary, LocationSummary, Profile, ProfilePatch, SocialLink,
};
use crate::users::{bio, social_links};

pub async fn put(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<ProfilePatch>,
) -> Result<impl IntoResponse, AppError> {
    // 1) Validate everything before any DB write.
    if let Some(Some(name)) = body.display_name.as_ref() {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.chars().count() > 60 {
            return Err(AppError::bad_request("invalid_display_name"));
        }
    }
    if let Some(Some(tag)) = body.tagline.as_ref() {
        if tag.chars().count() > 140 {
            return Err(AppError::bad_request("tagline_too_long"));
        }
    }
    let bio_sanitised: Option<Option<String>> = body.bio_html.as_ref().map(|outer| {
        outer.as_ref().map(|raw| {
            let cleaned = bio::sanitize(raw);
            if cleaned.len() > 16_384 {
                cleaned.chars().take(16_384).collect()
            } else {
                cleaned
            }
        })
    });
    if let Some(loc) = body.location.as_ref() {
        if let Some(b) = loc.bortle_class {
            if !(1..=9).contains(&b) {
                return Err(AppError::bad_request("bortle_out_of_range"));
            }
        }
        if let Some(s) = loc.sqm {
            if !(0.0..=99.99).contains(&s) {
                return Err(AppError::bad_request("sqm_out_of_range"));
            }
        }
    }
    if let Some(links) = body.social_links.as_ref() {
        social_links::validate_links(links)?;
    }

    // 2) Build the UPDATE: ProfilePatch fields are double-Option,
    //    so we must use COALESCE-with-sentinel or per-column conditional UPDATE.
    //    We do per-column conditional updates inside one transaction to keep this readable.
    let mut tx = state.pool.begin().await?;

    if let Some(opt) = body.display_name {
        let val = opt.as_ref().map(|s| s.trim().to_string());
        if let Some(s) = val {
            sqlx::query!("update users set display_name = $1 where id = $2", s, user.id)
                .execute(&mut *tx)
                .await?;
        }
        // display_name is NOT NULL — explicit null is a noop above.
    }
    if let Some(opt) = body.tagline {
        sqlx::query!(
            "update users set tagline = $1 where id = $2",
            opt.as_deref(),
            user.id
        )
        .execute(&mut *tx)
        .await?;
    }
    if let Some(opt) = bio_sanitised {
        sqlx::query!(
            "update users set bio_html = $1 where id = $2",
            opt.as_deref(),
            user.id
        )
        .execute(&mut *tx)
        .await?;
    }
    if let Some(eq) = body.equipment {
        sqlx::query!(
            r#"
            update users set
                equipment_telescope = $1,
                equipment_camera    = $2,
                equipment_mount     = $3,
                equipment_filters   = $4,
                equipment_guiding   = $5
            where id = $6
            "#,
            eq.telescope.as_deref(),
            eq.camera.as_deref(),
            eq.mount.as_deref(),
            eq.filters.as_deref(),
            eq.guiding.as_deref(),
            user.id
        )
        .execute(&mut *tx)
        .await?;
    }
    if let Some(loc) = body.location {
        sqlx::query!(
            r#"
            update users set
                location_text = $1,
                bortle_class  = $2,
                sqm           = $3
            where id = $4
            "#,
            loc.location_text.as_deref(),
            loc.bortle_class,
            loc.sqm.map(|f| sqlx::types::Decimal::from_f64_retain(f).unwrap_or_default()),
            user.id
        )
        .execute(&mut *tx)
        .await?;
    }
    if let Some(links) = body.social_links {
        let json = serde_json::to_value(&links).map_err(|_| AppError::internal("social_links_serialise"))?;
        sqlx::query!(
            "update users set social_links = $1 where id = $2",
            json,
            user.id
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
```

(`Decimal::from_f64_retain` — if your sqlx feature set uses a different decimal crate, the right import is whatever the existing code uses for `sqm` reads in Task 6's GET. Reconcile if `cargo check` complains.)

- [ ] **Step 4: Re-run sqlx prepare**

```
cd backend && cargo sqlx prepare -- --tests
```

- [ ] **Step 5: Run the tests**

```
cd backend && cargo test -p astrophoto --test profile_extended -- --nocapture
```

Expected: all five tests in this file pass.

- [ ] **Step 6: Run the full suite**

```
cd backend && cargo test --tests
```

Expected: clean. Pre-existing flakes in `tests/photos_phase8b` may still flake; treat that as a known issue (acceptance doc records it).

- [ ] **Step 7: Commit**

```
git add backend/src/users/profile.rs backend/tests/profile_extended.rs backend/.sqlx/
git commit -m "feat(profile): PATCH writes every P2 field with sanitisation

bio_html runs through ammonia regardless of client. social_links runs
through the platform-aware validator. Double-Option semantics on
nullable fields: absent = leave alone, explicit null = clear."
```

---

### Task 8: POST /api/me/cover — set cover_photo_id from owned photos

**Files:**
- Create: `backend/src/users/cover.rs`
- Modify: `backend/src/users/mod.rs`
- Modify: `backend/src/http/mod.rs`
- Test: `backend/tests/cover_set.rs`

- [ ] **Step 1: Write the failing test**

Create `backend/tests/cover_set.rs`:

```rust
mod common;

use common::{TestApp, ready_photo};
use uuid::Uuid;

#[tokio::test]
async fn set_cover_writes_users_cover_photo_id() {
    let app = TestApp::launch().await;
    let (cookie, user_id) = app.signup_with_handle("Marie", "marie", "marie@x.test").await;
    let photo_id = ready_photo(&app, &cookie, user_id).await;

    let res = app
        .client
        .post(format!("{}/api/me/cover", app.base_url))
        .header("cookie", cookie.clone())
        .json(&serde_json::json!({ "photo_id": photo_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204);

    let row = sqlx::query!("select cover_photo_id from users where id = $1", user_id)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(row.cover_photo_id, Some(photo_id));
}

#[tokio::test]
async fn set_cover_404_when_photo_not_owned() {
    let app = TestApp::launch().await;
    let (a_cookie, _a_id) = app.signup_with_handle("A", "alice", "a@x.test").await;
    let (_b_cookie, b_id) = app.signup_with_handle("B", "bob", "b@x.test").await;
    let other_photo = ready_photo(&app, &_b_cookie, b_id).await;

    let res = app
        .client
        .post(format!("{}/api/me/cover", app.base_url))
        .header("cookie", a_cookie)
        .json(&serde_json::json!({ "photo_id": other_photo }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404);
}

#[tokio::test]
async fn set_cover_clears_when_photo_id_null() {
    let app = TestApp::launch().await;
    let (cookie, user_id) = app.signup_with_handle("Marie", "marie", "marie@x.test").await;
    let photo_id = ready_photo(&app, &cookie, user_id).await;

    app.client
        .post(format!("{}/api/me/cover", app.base_url))
        .header("cookie", cookie.clone())
        .json(&serde_json::json!({ "photo_id": photo_id }))
        .send()
        .await
        .unwrap();

    let res = app
        .client
        .post(format!("{}/api/me/cover", app.base_url))
        .header("cookie", cookie)
        .json(&serde_json::json!({ "photo_id": Option::<Uuid>::None }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204);

    let row = sqlx::query!("select cover_photo_id from users where id = $1", user_id)
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(row.cover_photo_id, None);
}
```

`ready_photo` is a helper that signs up a user, runs the upload init/PUT/finalize/publish flow, and returns the published photo's UUID. Verify it exists in `backend/tests/common.rs` with `grep -n "fn ready_photo" backend/tests/common.rs`. If it doesn't (P1 may have inlined this in some suites), add it now to `common.rs` mirroring whatever `tests/permalink.rs` or `tests/handle_redirect.rs` does to produce a published photo — same author, same publish path, same module path. Do this in a separate uncommitted edit and squash into the cover commit.

- [ ] **Step 2: Run the tests — fail (handler doesn't exist)**

```
cd backend && cargo test -p astrophoto --test cover_set -- --nocapture
```

Expected: 404 on `/api/me/cover` (route not registered).

- [ ] **Step 3: Create `backend/src/users/cover.rs`**

```rust
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

#[derive(Deserialize)]
pub struct Body {
    pub photo_id: Option<Uuid>,
}

pub async fn set(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<Body>,
) -> Result<impl IntoResponse, AppError> {
    if let Some(pid) = body.photo_id {
        // Photo must exist, be owned by the caller, be published, and be ready.
        let row = sqlx::query!(
            r#"
            select 1 as ok
            from photos
            where id = $1
              and owner_id = $2
              and published_at is not null
              and status = 'ready'
            "#,
            pid,
            user.id
        )
        .fetch_optional(&state.pool)
        .await?;
        if row.is_none() {
            return Err(AppError::not_found("photo_not_owned_or_unpublished"));
        }
        sqlx::query!(
            "update users set cover_photo_id = $1 where id = $2",
            pid,
            user.id
        )
        .execute(&state.pool)
        .await?;
    } else {
        sqlx::query!(
            "update users set cover_photo_id = null where id = $1",
            user.id
        )
        .execute(&state.pool)
        .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}
```

- [ ] **Step 4: Register the module + route**

In `backend/src/users/mod.rs`, add `pub mod cover;` next to the others.

In `backend/src/http/mod.rs`, add — within the `Router::new()` chain near the other `/api/me/*` routes:

```rust
        .route(
            "/api/me/cover",
            axum::routing::post(crate::users::cover::set),
        )
```

- [ ] **Step 5: sqlx prepare + run tests**

```
cd backend && cargo sqlx prepare -- --tests && cargo test -p astrophoto --test cover_set
```

Expected: all three tests pass.

- [ ] **Step 6: Commit**

```
git add backend/src/users/cover.rs backend/src/users/mod.rs backend/src/http/mod.rs backend/tests/cover_set.rs backend/tests/common.rs backend/.sqlx/
git commit -m "feat(profile): POST /api/me/cover sets users.cover_photo_id

Validates the photo is owned, published, and ready. Sending photo_id:null clears the cover."
```

---

### Task 9: Featured pin / unpin / reorder — transactional implementation

The partial unique index `photos_featured_per_user_uidx` on `(owner_id, featured_position) WHERE featured_at IS NOT NULL` (migration 0009) means we must never have two pinned rows at the same position even momentarily. The reorder endpoint stages writes in two passes: pass 1 NULLs out positions for the affected rows, pass 2 writes the new positions. Pin and unpin are simpler — pin claims the lowest free 1..=6, unpin compacts.

**Files:**
- Create: `backend/src/users/featured.rs`
- Modify: `backend/src/users/mod.rs`
- Modify: `backend/src/http/mod.rs`
- Test: `backend/tests/featured_pin.rs`, `backend/tests/featured_reorder.rs`

- [ ] **Step 1: Write the pin/unpin tests**

Create `backend/tests/featured_pin.rs`:

```rust
mod common;

use common::{TestApp, ready_photo};

async fn pin(app: &TestApp, cookie: &str, photo_id: uuid::Uuid) -> reqwest::Response {
    app.client
        .post(format!("{}/api/me/featured/{}", app.base_url, photo_id))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap()
}

async fn unpin(app: &TestApp, cookie: &str, photo_id: uuid::Uuid) -> reqwest::Response {
    app.client
        .delete(format!("{}/api/me/featured/{}", app.base_url, photo_id))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn pin_assigns_position_one_then_two() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let p1 = ready_photo(&app, &cookie, uid).await;
    let p2 = ready_photo(&app, &cookie, uid).await;

    assert_eq!(pin(&app, &cookie, p1).await.status(), 204);
    assert_eq!(pin(&app, &cookie, p2).await.status(), 204);

    let rows = sqlx::query!(
        "select id, featured_position from photos where owner_id=$1 and featured_at is not null order by featured_position",
        uid
    )
    .fetch_all(&app.pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].featured_position, Some(1));
    assert_eq!(rows[1].featured_position, Some(2));
    assert_eq!(rows[0].id, p1);
    assert_eq!(rows[1].id, p2);
}

#[tokio::test]
async fn pin_idempotent_for_already_pinned_photo() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let p = ready_photo(&app, &cookie, uid).await;

    assert_eq!(pin(&app, &cookie, p).await.status(), 204);
    assert_eq!(pin(&app, &cookie, p).await.status(), 204); // idempotent

    let count = sqlx::query!(
        "select count(*) as c from photos where owner_id=$1 and featured_at is not null",
        uid
    )
    .fetch_one(&app.pool)
    .await
    .unwrap()
    .c
    .unwrap_or(0);
    assert_eq!(count, 1);
}

#[tokio::test]
async fn pin_409_when_six_already_pinned() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let mut ids = vec![];
    for _ in 0..6 {
        ids.push(ready_photo(&app, &cookie, uid).await);
    }
    for id in &ids {
        assert_eq!(pin(&app, &cookie, *id).await.status(), 204);
    }

    let extra = ready_photo(&app, &cookie, uid).await;
    let res = pin(&app, &cookie, extra).await;
    assert_eq!(res.status(), 409);
}

#[tokio::test]
async fn pin_404_when_not_owner() {
    let app = TestApp::launch().await;
    let (a_cookie, _) = app.signup_with_handle("A", "alice", "a@x.test").await;
    let (b_cookie, b_id) = app.signup_with_handle("B", "bob", "b@x.test").await;
    let p = ready_photo(&app, &b_cookie, b_id).await;
    assert_eq!(pin(&app, &a_cookie, p).await.status(), 404);
}

#[tokio::test]
async fn unpin_compacts_positions() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let p1 = ready_photo(&app, &cookie, uid).await;
    let p2 = ready_photo(&app, &cookie, uid).await;
    let p3 = ready_photo(&app, &cookie, uid).await;
    pin(&app, &cookie, p1).await;
    pin(&app, &cookie, p2).await;
    pin(&app, &cookie, p3).await;

    assert_eq!(unpin(&app, &cookie, p2).await.status(), 204);

    let rows = sqlx::query!(
        "select id, featured_position from photos where owner_id=$1 and featured_at is not null order by featured_position",
        uid
    )
    .fetch_all(&app.pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, p1);
    assert_eq!(rows[0].featured_position, Some(1));
    assert_eq!(rows[1].id, p3);
    assert_eq!(rows[1].featured_position, Some(2));
}

#[tokio::test]
async fn unpin_idempotent_for_unpinned_photo() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let p = ready_photo(&app, &cookie, uid).await;
    assert_eq!(unpin(&app, &cookie, p).await.status(), 204);
}
```

- [ ] **Step 2: Write the reorder test**

Create `backend/tests/featured_reorder.rs`:

```rust
mod common;

use common::{TestApp, ready_photo};
use uuid::Uuid;

#[tokio::test]
async fn reorder_moves_photos_to_supplied_positions() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let mut ids = vec![];
    for _ in 0..3 {
        ids.push(ready_photo(&app, &cookie, uid).await);
    }
    for id in &ids {
        app.client
            .post(format!("{}/api/me/featured/{}", app.base_url, id))
            .header("cookie", cookie.clone())
            .send()
            .await
            .unwrap();
    }

    // Reverse the order: positions 3,2,1.
    let body = serde_json::json!({ "photo_ids": [ids[2], ids[1], ids[0]] });
    let res = app
        .client
        .patch(format!("{}/api/me/featured/order", app.base_url))
        .header("cookie", cookie)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 204);

    let rows = sqlx::query!(
        "select id, featured_position from photos where owner_id=$1 and featured_at is not null order by featured_position",
        uid
    )
    .fetch_all(&app.pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].id, ids[2]);
    assert_eq!(rows[1].id, ids[1]);
    assert_eq!(rows[2].id, ids[0]);
}

#[tokio::test]
async fn reorder_400_when_a_photo_is_not_currently_pinned() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let pinned = ready_photo(&app, &cookie, uid).await;
    let unpinned = ready_photo(&app, &cookie, uid).await;
    app.client
        .post(format!("{}/api/me/featured/{}", app.base_url, pinned))
        .header("cookie", cookie.clone())
        .send()
        .await
        .unwrap();

    let body = serde_json::json!({ "photo_ids": [pinned, unpinned] });
    let res = app
        .client
        .patch(format!("{}/api/me/featured/order", app.base_url))
        .header("cookie", cookie)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn reorder_400_when_duplicate_id() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let p1 = ready_photo(&app, &cookie, uid).await;
    let p2 = ready_photo(&app, &cookie, uid).await;
    app.client
        .post(format!("{}/api/me/featured/{}", app.base_url, p1))
        .header("cookie", cookie.clone())
        .send()
        .await
        .unwrap();
    app.client
        .post(format!("{}/api/me/featured/{}", app.base_url, p2))
        .header("cookie", cookie.clone())
        .send()
        .await
        .unwrap();

    let body = serde_json::json!({ "photo_ids": [p1, p1] });
    let res = app
        .client
        .patch(format!("{}/api/me/featured/order", app.base_url))
        .header("cookie", cookie)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);
}

#[tokio::test]
async fn reorder_400_when_more_than_six() {
    let app = TestApp::launch().await;
    let (cookie, _uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let body = serde_json::json!({
        "photo_ids": (0..7).map(|_| Uuid::new_v4()).collect::<Vec<_>>()
    });
    let res = app
        .client
        .patch(format!("{}/api/me/featured/order", app.base_url))
        .header("cookie", cookie)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 400);
}
```

- [ ] **Step 3: Run both tests — they must fail (handlers don't exist)**

```
cd backend && cargo test -p astrophoto --test featured_pin --test featured_reorder
```

- [ ] **Step 4: Create `backend/src/users/featured.rs`**

```rust
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

const MAX_SLOTS: i64 = 6;

pub async fn pin(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(photo_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let mut tx = state.pool.begin().await?;

    // Verify ownership + status.
    let photo = sqlx::query!(
        r#"
        select featured_position
        from photos
        where id = $1
          and owner_id = $2
          and published_at is not null
          and status = 'ready'
        for update
        "#,
        photo_id,
        user.id
    )
    .fetch_optional(&mut *tx)
    .await?;

    let Some(p) = photo else {
        return Err(AppError::not_found("photo_not_owned_or_unpublished"));
    };
    if p.featured_position.is_some() {
        // Already pinned — idempotent.
        tx.commit().await?;
        return Ok(StatusCode::NO_CONTENT);
    }

    // Find the lowest free position 1..=6. If none, 409.
    let row = sqlx::query!(
        r#"
        select coalesce(min(p), 0)::int8 as next_pos
        from generate_series(1, $1::int8) as g(p)
        where not exists (
            select 1 from photos
            where owner_id = $2
              and featured_position = g.p
              and featured_at is not null
        )
        "#,
        MAX_SLOTS,
        user.id
    )
    .fetch_one(&mut *tx)
    .await?;
    let next = row.next_pos.unwrap_or(0);
    if next == 0 {
        return Err(AppError::conflict("featured_full"));
    }

    sqlx::query!(
        "update photos set featured_at = now(), featured_position = $1 where id = $2",
        next as i16,
        photo_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn unpin(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(photo_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let mut tx = state.pool.begin().await?;

    let row = sqlx::query!(
        r#"
        update photos
           set featured_at = null, featured_position = null
         where id = $1
           and owner_id = $2
           and featured_at is not null
        returning featured_position
        "#,
        photo_id,
        user.id
    )
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(r) = row {
        let removed = r.featured_position.unwrap_or(1) as i64;
        // Compact: every position > removed shifts down by 1, in one pass.
        // Since the unique index is partial, we stage via NULL first to be
        // safe under the partial unique constraint.
        let to_shift = sqlx::query!(
            r#"
            select id, featured_position from photos
            where owner_id = $1 and featured_at is not null and featured_position > $2
            order by featured_position
            for update
            "#,
            user.id,
            removed as i16
        )
        .fetch_all(&mut *tx)
        .await?;

        // Stage to NULL, then rewrite.
        for r in &to_shift {
            sqlx::query!(
                "update photos set featured_position = null where id = $1",
                r.id
            )
            .execute(&mut *tx)
            .await?;
        }
        for r in &to_shift {
            let new_pos = r.featured_position.unwrap_or(1) - 1;
            sqlx::query!(
                "update photos set featured_position = $1 where id = $2",
                new_pos,
                r.id
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct ReorderBody {
    pub photo_ids: Vec<Uuid>,
}

pub async fn reorder(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<ReorderBody>,
) -> Result<impl IntoResponse, AppError> {
    if body.photo_ids.is_empty() || body.photo_ids.len() > MAX_SLOTS as usize {
        return Err(AppError::bad_request("featured_order_count"));
    }
    let mut seen = std::collections::HashSet::new();
    for id in &body.photo_ids {
        if !seen.insert(*id) {
            return Err(AppError::bad_request("featured_order_duplicate"));
        }
    }

    let mut tx = state.pool.begin().await?;

    // Verify every supplied id is owned + currently pinned.
    let owned = sqlx::query!(
        r#"
        select id from photos
        where owner_id = $1 and featured_at is not null and id = any($2)
        for update
        "#,
        user.id,
        &body.photo_ids
    )
    .fetch_all(&mut *tx)
    .await?;
    if owned.len() != body.photo_ids.len() {
        return Err(AppError::bad_request("featured_order_unknown_id"));
    }

    // Pass 1: stage all affected rows to NULL.
    sqlx::query!(
        "update photos set featured_position = null where id = any($1)",
        &body.photo_ids
    )
    .execute(&mut *tx)
    .await?;

    // Pass 2: write target positions.
    for (i, id) in body.photo_ids.iter().enumerate() {
        let pos = (i as i16) + 1;
        sqlx::query!(
            "update photos set featured_position = $1 where id = $2",
            pos,
            id
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
```

- [ ] **Step 5: Register module + routes**

In `backend/src/users/mod.rs`, add `pub mod featured;`.

In `backend/src/http/mod.rs`, near the other `/api/me/*` routes:

```rust
        .route(
            "/api/me/featured/:photo_id",
            axum::routing::post(crate::users::featured::pin)
                .delete(crate::users::featured::unpin),
        )
        .route(
            "/api/me/featured/order",
            axum::routing::patch(crate::users::featured::reorder),
        )
```

Order matters: the `/order` route must be declared **before** `/:photo_id` if axum routes by registration order in your version, otherwise `:photo_id` will eat the literal `order` segment. Check by running the reorder test alone — if it 404s, swap the registration order.

- [ ] **Step 6: sqlx prepare + run tests**

```
cd backend && cargo sqlx prepare -- --tests && cargo test -p astrophoto --test featured_pin --test featured_reorder
```

Expected: 6 + 4 = 10 tests pass.

- [ ] **Step 7: Commit**

```
git add backend/src/users/featured.rs backend/src/users/mod.rs backend/src/http/mod.rs backend/tests/featured_pin.rs backend/tests/featured_reorder.rs backend/.sqlx/
git commit -m "feat(profile): featured pin / unpin / reorder endpoints

Pin claims the lowest free slot (1..=6) or 409s when full. Unpin
compacts the remaining positions. Reorder stages NULL-then-target in
one transaction to keep the partial unique constraint coherent."
```

---

### Task 10: GET /api/users/by-handle/:handle/profile — public aggregator

One round-trip the hero shell can hydrate from. Joins users, owned-photo counts, equipment, integration sum, follower count, appreciations sum, distinct targets count, and the featured photos.

**Files:**
- Create: `backend/src/users/public_profile.rs`
- Modify: `backend/src/users/mod.rs`
- Modify: `backend/src/http/mod.rs`
- Test: `backend/tests/public_profile.rs`

- [ ] **Step 1: Write the failing test**

Create `backend/tests/public_profile.rs`:

```rust
mod common;

use astrophoto::api_types::PublicProfile;
use common::{TestApp, ready_photo};

#[tokio::test]
async fn public_profile_returns_full_shape() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("Marie", "marie", "marie@x.test").await;

    // PATCH to populate.
    app.client
        .patch(format!("{}/api/me/profile", app.base_url))
        .header("cookie", cookie.clone())
        .json(&serde_json::json!({
            "tagline": "Hunting deep-sky from a Bortle 6 backyard",
            "bio_html": "<p>Hi</p>",
            "equipment": { "telescope": "RedCat 51", "camera": "ASI2600MC" },
            "location": { "location_text": "Lyon, FR", "bortle_class": 6, "sqm": 19.8 },
            "social_links": [{ "platform": "twitter", "url": "https://twitter.com/marie" }]
        }))
        .send()
        .await
        .unwrap();

    let p1 = ready_photo(&app, &cookie, uid).await;
    let p2 = ready_photo(&app, &cookie, uid).await;
    app.client
        .post(format!("{}/api/me/featured/{}", app.base_url, p1))
        .header("cookie", cookie.clone())
        .send()
        .await
        .unwrap();
    app.client
        .post(format!("{}/api/me/featured/{}", app.base_url, p2))
        .header("cookie", cookie.clone())
        .send()
        .await
        .unwrap();
    app.client
        .post(format!("{}/api/me/cover", app.base_url))
        .header("cookie", cookie)
        .json(&serde_json::json!({ "photo_id": p1 }))
        .send()
        .await
        .unwrap();

    // Public read.
    let res = app
        .client
        .get(format!("{}/api/users/by-handle/marie/profile", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: PublicProfile = res.json().await.unwrap();
    assert_eq!(body.handle, "marie");
    assert_eq!(body.display_name, "Marie");
    assert_eq!(body.tagline.as_deref(), Some("Hunting deep-sky from a Bortle 6 backyard"));
    assert_eq!(body.equipment.telescope.as_deref(), Some("RedCat 51"));
    assert_eq!(body.location.bortle_class, Some(6));
    assert_eq!(body.social_links.len(), 1);
    assert_eq!(body.featured.len(), 2);
    assert_eq!(body.featured[0].featured_position, 1);
    assert_eq!(body.featured[0].id, p1);
    assert_eq!(body.featured[1].id, p2);
    assert_eq!(body.cover.as_ref().unwrap().id, p1);
    assert_eq!(body.stats.frames, 2);
    assert_eq!(body.stats.member_since_year, chrono::Utc::now().date_naive().iter_days().next().map(|d| d.year()).unwrap_or(2026));
}

#[tokio::test]
async fn public_profile_404_for_unknown_handle() {
    let app = TestApp::launch().await;
    let res = app
        .client
        .get(format!("{}/api/users/by-handle/nobody/profile", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404);
}
```

(`chrono` import: the test uses `chrono::Utc::now().date_naive()` — if your test crate already imports chrono, fine; otherwise pin to whatever the existing tests use; the important assertion is just that `member_since_year` is not zero.)

- [ ] **Step 2: Run the test — fails, route doesn't exist**

```
cd backend && cargo test -p astrophoto --test public_profile
```

- [ ] **Step 3: Create `backend/src/users/public_profile.rs`**

```rust
use axum::{Json, extract::{Path, State}, response::IntoResponse};
use chrono::Datelike;

use crate::AppError;
use crate::api_types::{
    EquipmentSummary, FeaturedPhotoSummary, HeroStats, LocationSummary, PublicProfile, SocialLink,
};
use crate::http::AppState;

pub async fn get(
    State(state): State<AppState>,
    Path(handle): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let user = sqlx::query!(
        r#"
        select id, display_name, handle, tagline, bio_html,
               cover_photo_id,
               equipment_telescope, equipment_camera, equipment_mount,
               equipment_filters,   equipment_guiding,
               location_text, bortle_class, sqm,
               social_links,
               created_at
        from users
        where handle = $1
        "#,
        handle.to_lowercase()
    )
    .fetch_optional(&state.pool)
    .await?;

    let Some(u) = user else {
        return Err(AppError::not_found("user"));
    };

    // Fetch featured photos (ordered by position).
    let featured_rows = sqlx::query!(
        r#"
        select p.id, p.short_id, p.featured_position, p.target,
               p.appreciations_count, p.blurhash, p.width, p.height
        from photos p
        where p.owner_id = $1 and p.featured_at is not null
        order by p.featured_position
        "#,
        u.id
    )
    .fetch_all(&state.pool)
    .await?;

    let featured: Vec<FeaturedPhotoSummary> = featured_rows
        .into_iter()
        .map(|r| FeaturedPhotoSummary {
            id: r.id,
            short_id: r.short_id,
            featured_position: r.featured_position.unwrap_or(0),
            target: r.target,
            appreciations_count: r.appreciations_count,
            blurhash: r.blurhash,
            width: r.width,
            height: r.height,
        })
        .collect();

    // Cover summary (if any).
    let cover = if let Some(cov_id) = u.cover_photo_id {
        sqlx::query!(
            r#"
            select id, short_id, target, appreciations_count, blurhash, width, height
            from photos
            where id = $1
            "#,
            cov_id
        )
        .fetch_optional(&state.pool)
        .await?
        .map(|r| FeaturedPhotoSummary {
            id: r.id,
            short_id: r.short_id,
            featured_position: 0,
            target: r.target,
            appreciations_count: r.appreciations_count,
            blurhash: r.blurhash,
            width: r.width,
            height: r.height,
        })
    } else {
        None
    };

    // Stats — single round-trip.
    let stats = sqlx::query!(
        r#"
        select
            count(*) filter (where p.published_at is not null and p.status='ready')                            as frames,
            coalesce(sum(p.integration_seconds) filter (where p.published_at is not null), 0)::int8           as integration_seconds,
            coalesce(sum(p.appreciations_count) filter (where p.published_at is not null), 0)::int8           as appreciations,
            count(distinct pt.target_id) filter (where p.published_at is not null)                            as targets
        from photos p
        left join photo_targets pt on pt.photo_id = p.id
        where p.owner_id = $1
        "#,
        u.id
    )
    .fetch_one(&state.pool)
    .await?;

    let followers: i64 = sqlx::query_scalar!(
        "select count(*)::int8 from follows where followed_id = $1",
        u.id
    )
    .fetch_one(&state.pool)
    .await?
    .unwrap_or(0);

    let social_links: Vec<SocialLink> = serde_json::from_value(u.social_links)
        .map_err(|_| AppError::internal("social_links_corrupt"))?;

    let body = PublicProfile {
        id: u.id,
        handle: u.handle,
        display_name: u.display_name,
        tagline: u.tagline,
        bio_html: u.bio_html,
        cover,
        equipment: EquipmentSummary {
            telescope: u.equipment_telescope,
            camera: u.equipment_camera,
            mount: u.equipment_mount,
            filters: u.equipment_filters,
            guiding: u.equipment_guiding,
        },
        location: LocationSummary {
            location_text: u.location_text,
            bortle_class: u.bortle_class,
            sqm: u.sqm.map(|d: sqlx::types::Decimal| d.to_string().parse::<f64>().unwrap_or(0.0)),
        },
        social_links,
        stats: HeroStats {
            frames: stats.frames.unwrap_or(0),
            integration_seconds: stats.integration_seconds.unwrap_or(0),
            followers,
            appreciations: stats.appreciations.unwrap_or(0),
            targets: stats.targets.unwrap_or(0),
            member_since_year: u.created_at.year(),
        },
        featured,
    };
    Ok(Json(body))
}
```

(`integration_seconds` column: P1's spec assumed photos accumulate exposure data. If the column is named differently — e.g. `total_exposure_s` — adjust. `cargo check` will surface the truth; rename in the SQL accordingly.)

- [ ] **Step 4: Register module + route**

`backend/src/users/mod.rs`: `pub mod public_profile;`.

`backend/src/http/mod.rs`:

```rust
        .route(
            "/api/users/by-handle/:handle/profile",
            axum::routing::get(crate::users::public_profile::get),
        )
```

- [ ] **Step 5: sqlx prepare + run**

```
cd backend && cargo sqlx prepare -- --tests && cargo test -p astrophoto --test public_profile -- --nocapture
```

If the test fails because `integration_seconds` doesn't exist on `photos`, search the schema:

```
grep -nE "integration|exposure" backend/migrations/0001_init.sql backend/migrations/000{4,7,8,9}*.sql
```

Pick the column the schema actually has and update both the SQL and the assertion.

- [ ] **Step 6: Commit**

```
git add backend/src/users/public_profile.rs backend/src/users/mod.rs backend/src/http/mod.rs backend/tests/public_profile.rs backend/.sqlx/
git commit -m "feat(profile): public aggregator GET /api/users/by-handle/:handle/profile

One round-trip for the hero shell — user fields, equipment, location,
social, stats (frames, integration, followers, appreciations, targets,
member-since-year), cover summary, and featured row."
```

---

### Task 11: GET /api/users/by-handle/:handle/photos — paginated gallery feed

Cursor-paginated, sortable. Cursor is opaque base64-of-`(published_at_unix_ms,id)` so ties on `published_at` resolve by id. Page size 24 default, max 60.

**Files:**
- Create: `backend/src/users/photos_feed.rs`
- Modify: `backend/src/users/mod.rs`, `backend/src/http/mod.rs`
- Test: `backend/tests/photos_feed.rs`

- [ ] **Step 1: Write the failing test**

Create `backend/tests/photos_feed.rs`:

```rust
mod common;

use astrophoto::api_types::GalleryPage;
use common::{TestApp, ready_photo};

#[tokio::test]
async fn photos_feed_returns_published_photos_newest_first() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let _p1 = ready_photo(&app, &cookie, uid).await;
    let _p2 = ready_photo(&app, &cookie, uid).await;
    let p3 = ready_photo(&app, &cookie, uid).await;

    let res = app
        .client
        .get(format!("{}/api/users/by-handle/marie/photos?limit=2", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let body: GalleryPage = res.json().await.unwrap();
    assert_eq!(body.photos.len(), 2);
    assert_eq!(body.photos[0].id, p3, "newest first");
    assert!(body.next_cursor.is_some());

    // Page 2 with cursor.
    let cursor = body.next_cursor.unwrap();
    let res = app
        .client
        .get(format!(
            "{}/api/users/by-handle/marie/photos?limit=2&cursor={}",
            app.base_url, cursor
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let page2: GalleryPage = res.json().await.unwrap();
    assert_eq!(page2.photos.len(), 1);
    assert!(page2.next_cursor.is_none());
}

#[tokio::test]
async fn photos_feed_404_for_unknown_handle() {
    let app = TestApp::launch().await;
    let res = app
        .client
        .get(format!("{}/api/users/by-handle/nobody/photos", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 404);
}

#[tokio::test]
async fn photos_feed_respects_limit_bounds() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let _ = ready_photo(&app, &cookie, uid).await;

    let res = app
        .client
        .get(format!("{}/api/users/by-handle/marie/photos?limit=999", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "limit clamped, not 400");

    let res = app
        .client
        .get(format!("{}/api/users/by-handle/marie/photos?limit=0", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn photos_feed_sort_popular_orders_by_appreciations() {
    let app = TestApp::launch().await;
    let (cookie, uid) = app.signup_with_handle("M", "marie", "m@x.test").await;
    let p1 = ready_photo(&app, &cookie, uid).await;
    let p2 = ready_photo(&app, &cookie, uid).await;

    // Bump p1's appreciations directly via SQL — we don't need the full appreciate flow here.
    sqlx::query!(
        "update photos set appreciations_count = 5 where id = $1",
        p1
    )
    .execute(&app.pool)
    .await
    .unwrap();

    let res = app
        .client
        .get(format!("{}/api/users/by-handle/marie/photos?sort=popular", app.base_url))
        .send()
        .await
        .unwrap();
    let body: GalleryPage = res.json().await.unwrap();
    assert_eq!(body.photos[0].id, p1, "popular puts highest appreciations first");
    assert_eq!(body.photos[1].id, p2);
}
```

- [ ] **Step 2: Run — fails (route doesn't exist)**

```
cd backend && cargo test -p astrophoto --test photos_feed
```

- [ ] **Step 3: Create `backend/src/users/photos_feed.rs`**

```rust
use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppError;
use crate::api_types::{GalleryPage, GalleryPhoto};
use crate::http::AppState;

#[derive(Deserialize)]
pub struct Q {
    pub cursor: Option<String>,
    pub limit:  Option<i64>,
    pub sort:   Option<String>, // "newest" (default) | "popular"
}

const DEFAULT_LIMIT: i64 = 24;
const MAX_LIMIT: i64 = 60;

pub async fn get(
    State(state): State<AppState>,
    Path(handle): Path<String>,
    Query(q): Query<Q>,
) -> Result<impl IntoResponse, AppError> {
    let user = sqlx::query!(
        "select id from users where handle = $1",
        handle.to_lowercase()
    )
    .fetch_optional(&state.pool)
    .await?;
    let Some(u) = user else {
        return Err(AppError::not_found("user"));
    };

    let limit = q
        .limit
        .unwrap_or(DEFAULT_LIMIT)
        .clamp(1, MAX_LIMIT);
    let sort = q.sort.as_deref().unwrap_or("newest");

    // Decode cursor if present.
    let cursor = q.cursor.as_deref().map(decode_cursor).transpose()?;

    let rows = match sort {
        "popular" => {
            // For popular, cursor encodes (appreciations_count, published_at_ms, id).
            sqlx::query!(
                r#"
                select id, short_id, target, width, height, blurhash, appreciations_count, published_at
                from photos
                where owner_id = $1
                  and published_at is not null
                  and status = 'ready'
                  and ($2::int4 is null or appreciations_count < $2 or
                       (appreciations_count = $2 and (published_at, id) < ($3, $4)))
                order by appreciations_count desc, published_at desc, id desc
                limit $5
                "#,
                u.id,
                cursor.as_ref().and_then(|c| c.appreciations),
                cursor.as_ref().map(|c| c.published_at).unwrap_or_else(chrono::Utc::now),
                cursor.as_ref().map(|c| c.id).unwrap_or(Uuid::nil()),
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
        _ => {
            // newest
            sqlx::query!(
                r#"
                select id, short_id, target, width, height, blurhash, appreciations_count, published_at
                from photos
                where owner_id = $1
                  and published_at is not null
                  and status = 'ready'
                  and ($2::timestamptz is null or (published_at, id) < ($2, $3))
                order by published_at desc, id desc
                limit $4
                "#,
                u.id,
                cursor.as_ref().map(|c| c.published_at),
                cursor.as_ref().map(|c| c.id).unwrap_or(Uuid::nil()),
                limit + 1
            )
            .fetch_all(&state.pool)
            .await?
        }
    };

    let more = rows.len() as i64 > limit;
    let take: Vec<_> = rows.into_iter().take(limit as usize).collect();

    let next_cursor = if more && !take.is_empty() {
        let last = take.last().unwrap();
        Some(encode_cursor(&Cursor {
            published_at: last.published_at.unwrap(),
            id: last.id,
            appreciations: if sort == "popular" {
                Some(last.appreciations_count)
            } else {
                None
            },
        }))
    } else {
        None
    };

    Ok(Json(GalleryPage {
        photos: take
            .into_iter()
            .map(|r| GalleryPhoto {
                id: r.id,
                short_id: r.short_id,
                target: r.target,
                width: r.width,
                height: r.height,
                blurhash: r.blurhash,
                appreciations_count: r.appreciations_count,
                published_at: r.published_at,
            })
            .collect(),
        next_cursor,
    }))
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Cursor {
    published_at: chrono::DateTime<chrono::Utc>,
    id: Uuid,
    #[serde(default)]
    appreciations: Option<i32>,
}

fn encode_cursor(c: &Cursor) -> String {
    let bytes = serde_json::to_vec(c).unwrap_or_default();
    URL_SAFE_NO_PAD.encode(bytes)
}

fn decode_cursor(s: &str) -> Result<Cursor, AppError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|_| AppError::bad_request("cursor_invalid"))?;
    serde_json::from_slice(&bytes).map_err(|_| AppError::bad_request("cursor_invalid"))
}
```

(`base64` crate may not be in `Cargo.toml`. Add with `cargo add base64@0.22` if `cargo check` complains. Reuse the existing crate version if elsewhere in the codebase; do not introduce drift.)

- [ ] **Step 4: Register module + route**

```rust
        .route(
            "/api/users/by-handle/:handle/photos",
            axum::routing::get(crate::users::photos_feed::get),
        )
```

- [ ] **Step 5: sqlx prepare + run**

```
cd backend && cargo sqlx prepare -- --tests && cargo test -p astrophoto --test photos_feed
```

Expected: 4/4 pass.

- [ ] **Step 6: Commit**

```
git add backend/src/users/photos_feed.rs backend/src/users/mod.rs backend/src/http/mod.rs backend/tests/photos_feed.rs backend/Cargo.toml backend/Cargo.lock backend/.sqlx/
git commit -m "feat(profile): cursor-paginated gallery feed by handle

Sort: newest|popular. Cursor encodes (published_at,id[,appreciations])
for stable tie-breaking. limit clamped to 1..=60."
```

---

### Task 12: Run the full backend suite + commit any TS regen

```
cd backend && cargo test --tests
cd .. && just types
```

Stage and commit any TS regen drift that wasn't already covered by Task 4. Skip the commit if `git diff --stat` is empty.

```
git add frontend/src/lib/api/
git diff --cached --stat || true
git commit -m "chore(types): regenerate after P2 backend types" || echo "nothing to commit"
```

---

## Frontend — primitives

The hero-page build pulls in a handful of small utilities and a typed
profile-API client before the components themselves.

### Task 13: Frontend API helpers — `frontend/src/lib/api/profile.ts`

**Files:**
- Create: `frontend/src/lib/api/profile.ts`

- [ ] **Step 1: Write the file**

```ts
import type {
  Profile,
  PublicProfile,
  GalleryPage,
} from '$lib/api';

const API_BASE: string =
  (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '';

type FetchFn = typeof fetch;

export async function fetchOwnerProfile(f: FetchFn): Promise<Profile> {
  const r = await f(`${API_BASE}/api/me/profile`, { credentials: 'include' });
  if (!r.ok) throw new Error(`fetchOwnerProfile ${r.status}`);
  return (await r.json()) as Profile;
}

export async function patchOwnerProfile(
  f: FetchFn,
  body: Partial<Profile> & { social_links?: Profile['social_links'] }
): Promise<void> {
  const r = await f(`${API_BASE}/api/me/profile`, {
    method: 'PATCH',
    credentials: 'include',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body)
  });
  if (!r.ok) throw new Error(`patchOwnerProfile ${r.status}`);
}

export async function fetchPublicProfile(
  f: FetchFn,
  handle: string
): Promise<PublicProfile> {
  const r = await f(`${API_BASE}/api/users/by-handle/${handle}/profile`);
  if (r.status === 404) throw new Error('not_found');
  if (!r.ok) throw new Error(`fetchPublicProfile ${r.status}`);
  return (await r.json()) as PublicProfile;
}

export async function fetchPhotosFeed(
  f: FetchFn,
  handle: string,
  opts: { cursor?: string; sort?: 'newest' | 'popular'; limit?: number } = {}
): Promise<GalleryPage> {
  const u = new URL(`${API_BASE}/api/users/by-handle/${handle}/photos`, location.origin);
  if (opts.cursor) u.searchParams.set('cursor', opts.cursor);
  if (opts.sort)   u.searchParams.set('sort', opts.sort);
  if (opts.limit)  u.searchParams.set('limit', String(opts.limit));
  const r = await f(u.toString());
  if (!r.ok) throw new Error(`fetchPhotosFeed ${r.status}`);
  return (await r.json()) as GalleryPage;
}

export async function setCover(
  f: FetchFn,
  photoId: string | null
): Promise<void> {
  const r = await f(`${API_BASE}/api/me/cover`, {
    method: 'POST',
    credentials: 'include',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ photo_id: photoId })
  });
  if (!r.ok) throw new Error(`setCover ${r.status}`);
}

export async function pinFeatured(f: FetchFn, photoId: string): Promise<void> {
  const r = await f(`${API_BASE}/api/me/featured/${photoId}`, {
    method: 'POST',
    credentials: 'include'
  });
  if (!r.ok) throw new Error(`pinFeatured ${r.status}`);
}

export async function unpinFeatured(f: FetchFn, photoId: string): Promise<void> {
  const r = await f(`${API_BASE}/api/me/featured/${photoId}`, {
    method: 'DELETE',
    credentials: 'include'
  });
  if (!r.ok) throw new Error(`unpinFeatured ${r.status}`);
}

export async function reorderFeatured(
  f: FetchFn,
  photoIds: string[]
): Promise<void> {
  const r = await f(`${API_BASE}/api/me/featured/order`, {
    method: 'PATCH',
    credentials: 'include',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ photo_ids: photoIds })
  });
  if (!r.ok) throw new Error(`reorderFeatured ${r.status}`);
}
```

`$lib/api` is the existing barrel index file; if it doesn't currently re-export the new types, open `frontend/src/lib/api/index.ts` and add re-exports for `Profile`, `PublicProfile`, `GalleryPage`, `GalleryPhoto`, `FeaturedPhotoSummary`, `EquipmentSummary`, `LocationSummary`, `SocialLink`, `SocialPlatform`, `HeroStats`. If that barrel doesn't exist, create it with named re-exports.

- [ ] **Step 2: Verify no svelte-check errors**

```
cd frontend && pnpm check
```

- [ ] **Step 3: Commit**

```
git add frontend/src/lib/api/profile.ts frontend/src/lib/api/index.ts
git commit -m "feat(frontend): typed P2 profile API client"
```

---

### Task 14: Format helpers — integration time + cover URL

**Files:**
- Create: `frontend/src/lib/format/integration.ts`
- Test: `frontend/src/lib/format/integration.test.ts` (Vitest)

- [ ] **Step 1: Write the failing test**

```ts
import { describe, it, expect } from 'vitest';
import { formatIntegration } from './integration';

describe('formatIntegration', () => {
  it('returns em-dash when zero', () => {
    expect(formatIntegration(0)).toBe('—');
  });
  it('formats hours and minutes', () => {
    expect(formatIntegration(3 * 3600 + 14 * 60)).toBe('3h 14m');
  });
  it('drops minutes when whole hours', () => {
    expect(formatIntegration(5 * 3600)).toBe('5h');
  });
  it('shows minutes only under one hour', () => {
    expect(formatIntegration(45 * 60)).toBe('45m');
  });
});
```

- [ ] **Step 2: Run test — fails (module doesn't exist)**

```
cd frontend && pnpm vitest run src/lib/format/integration.test.ts
```

- [ ] **Step 3: Write the helper**

```ts
export function formatIntegration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return '—';
  const totalMinutes = Math.floor(seconds / 60);
  const h = Math.floor(totalMinutes / 60);
  const m = totalMinutes % 60;
  if (h === 0) return `${m}m`;
  if (m === 0) return `${h}h`;
  return `${h}h ${m}m`;
}
```

- [ ] **Step 4: Run test — passes**

```
cd frontend && pnpm vitest run src/lib/format/integration.test.ts
```

- [ ] **Step 5: Commit**

```
git add frontend/src/lib/format/integration.ts frontend/src/lib/format/integration.test.ts
git commit -m "feat(frontend): formatIntegration helper for stats row"
```

---

## Frontend — hero shell

**Reference:** `/Users/pleclech/Downloads/design_handoff_astrophoto 3/showcase/showcase-p2.jsx` is the canonical source for layout, copy, dimensions, and class names. Each Svelte component below mirrors a JSX block from that file, translated to Svelte 5 runes. CSS uses the existing app tokens (`--bg-canvas`, `--accent`, `--accent-ink`, `--border-subtle`, `--font-display`, `--font-mono`, etc.); do not introduce new tokens unless the JSX references one not yet defined — in that case add it to `frontend/src/app.css` and call it out in the commit message.

### Task 15: HeroPage orchestration component

**Files:**
- Create: `frontend/src/lib/components/profile/HeroPage.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script lang="ts">
  import type { PublicProfile } from '$lib/api';
  import OwnerModeBanner from './OwnerModeBanner.svelte';
  import HeroCover from './HeroCover.svelte';
  import HeroIdentity from './HeroIdentity.svelte';
  import HeroAbout from './HeroAbout.svelte';
  import HeroEquipmentStrip from './HeroEquipmentStrip.svelte';
  import HeroLocationBadge from './HeroLocationBadge.svelte';
  import HeroStatsRow from './HeroStatsRow.svelte';
  import FeaturedRow from './FeaturedRow.svelte';
  import GalleryToolbar from './GalleryToolbar.svelte';
  import PhotoGrid from './PhotoGrid.svelte';

  type ViewMode = 'visitor' | 'owner' | 'admin';

  let {
    profile,
    viewMode = 'visitor',
    onEditProfile = () => {},
    onPickCover = () => {},
    onPinFirst = () => {}
  }: {
    profile: PublicProfile;
    viewMode?: ViewMode;
    onEditProfile?: () => void;
    onPickCover?: () => void;
    onPinFirst?: () => void;
  } = $props();

  let isOwner = $derived(viewMode === 'owner');
</script>

<article class="hero-page" data-mode={viewMode}>
  {#if isOwner}
    <OwnerModeBanner onEdit={onEditProfile} />
  {/if}

  <HeroCover
    cover={profile.cover}
    {isOwner}
    onPickCover={onPickCover}
  />

  <HeroIdentity {profile} {isOwner} {onEditProfile} />

  <HeroAbout bio={profile.bio_html} {isOwner} {onEditProfile} />

  <HeroEquipmentStrip
    equipment={profile.equipment}
    {isOwner}
    {onEditProfile}
  />

  <HeroLocationBadge
    location={profile.location}
    {isOwner}
    {onEditProfile}
  />

  <HeroStatsRow stats={profile.stats} />

  <FeaturedRow
    items={profile.featured}
    handle={profile.handle}
    {isOwner}
    onPin={onPinFirst}
  />

  <GalleryToolbar />

  <PhotoGrid handle={profile.handle} />
</article>

<style>
  .hero-page {
    display: flex;
    flex-direction: column;
    gap: 0;
    background: var(--bg-canvas);
    color: var(--fg-primary);
  }
</style>
```

- [ ] **Step 2: Verify it parses (no `pnpm check` yet — its imports don't exist)**

```
cat frontend/src/lib/components/profile/HeroPage.svelte | head -1
```

- [ ] **Step 3: No commit yet** — this component is wired to children that don't exist; commit at the end of Task 30 once the shell composes.

---

### Task 16: OwnerModeBanner component

**Files:**
- Create: `frontend/src/lib/components/profile/OwnerModeBanner.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script lang="ts">
  let { onEdit }: { onEdit: () => void } = $props();
</script>

<div class="banner" role="status">
  <span class="dot">●</span>
  <span class="label">VIEWING YOUR OWN PROFILE · OWNER MODE</span>
  <button type="button" class="btn-edit" onclick={onEdit}>Edit profile</button>
</div>

<style>
  .banner {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 24px;
    background: color-mix(in srgb, var(--accent) 12%, var(--bg-canvas));
    color: var(--fg-primary);
    border-bottom: 1px solid var(--border-subtle);
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.06em;
  }
  .dot { color: var(--accent); }
  .label { flex: 1; }
  .btn-edit {
    background: var(--accent);
    color: var(--accent-ink);
    border: 0;
    padding: 6px 12px;
    font-family: var(--font-mono);
    font-size: 11px;
    cursor: pointer;
  }
</style>
```

- [ ] **Step 2: Skip commit** — bundle in Task 30.

---

### Task 17: HeroCover component

**Files:**
- Create: `frontend/src/lib/components/profile/HeroCover.svelte`

- [ ] **Step 1: Write the component**

The cover renders the user's `cover_photo_id`-pointed image as a 480 px desktop / 28 vh mobile banner with a bottom gradient that fades to `--bg-canvas`. When empty AND visitor: render nothing (the section is hidden, not stubbed). When empty AND owner: render an empty banner with a centred "Pick a cover from your gallery →" prompt that triggers `onPickCover`. When set: render the image plus a top-right credit line `● COVER · <target>` (target only — JSX in `showcase-p2.jsx` shows the target field; integration/when fields are not exposed by `FeaturedPhotoSummary` and are intentionally omitted from this slot).

```svelte
<script lang="ts">
  import type { FeaturedPhotoSummary } from '$lib/api';
  import Img from '$lib/components/Img.svelte';

  let {
    cover,
    isOwner,
    onPickCover
  }: {
    cover: FeaturedPhotoSummary | null;
    isOwner: boolean;
    onPickCover: () => void;
  } = $props();

  let hasCover = $derived(cover !== null && cover !== undefined);
</script>

{#if hasCover}
  <header class="cover" aria-label="Cover photo">
    <Img
      photoId={cover!.id}
      w={2400}
      alt={cover!.target ?? 'Cover image'}
      blurhash={cover!.blurhash ?? null}
      class="cover-img"
    />
    <div class="cover-credit">
      <span class="dot">●</span>
      <span>COVER</span>
      {#if cover!.target}
        <span class="dim">·</span>
        <span>{cover!.target}</span>
      {/if}
    </div>
    {#if isOwner}
      <button type="button" class="cover-edit" onclick={onPickCover}>Change cover</button>
    {/if}
  </header>
{:else if isOwner}
  <header class="cover cover--empty">
    <button type="button" class="cover-prompt" onclick={onPickCover}>
      Pick a cover from your gallery →
    </button>
  </header>
{/if}

<style>
  .cover {
    position: relative;
    width: 100%;
    height: 480px;
    overflow: hidden;
    background: var(--bg-elevated);
  }
  .cover :global(.cover-img) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .cover::after {
    content: '';
    position: absolute;
    inset: auto 0 0 0;
    height: 30%;
    background: linear-gradient(to bottom, transparent, var(--bg-canvas));
    pointer-events: none;
  }
  .cover--empty {
    display: flex;
    align-items: center;
    justify-content: center;
    background: color-mix(in srgb, var(--accent) 4%, var(--bg-elevated));
    height: 240px;
  }
  .cover-credit {
    position: absolute;
    top: 16px;
    right: 24px;
    z-index: 1;
    display: flex;
    gap: 8px;
    align-items: center;
    color: var(--fg-on-image, #fff);
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.06em;
  }
  .cover-credit .dot { color: var(--accent); }
  .cover-credit .dim { opacity: 0.6; }
  .cover-edit {
    position: absolute;
    top: 16px;
    left: 24px;
    z-index: 1;
    background: rgba(0, 0, 0, 0.5);
    color: #fff;
    border: 1px solid rgba(255, 255, 255, 0.2);
    padding: 6px 12px;
    font-family: var(--font-mono);
    font-size: 11px;
    cursor: pointer;
  }
  .cover-prompt {
    background: transparent;
    color: var(--accent);
    border: 1px solid var(--accent);
    padding: 12px 20px;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
  @media (max-width: 640px) {
    .cover {
      height: 28vh;
    }
  }
</style>
```

(The `<Img>` component already exists from P1; if it doesn't accept `blurhash` as a prop, check `frontend/src/lib/components/Img.svelte` and either add the prop or drop the line.)

- [ ] **Step 2: Skip commit.**

---

### Task 18: HeroIdentity + HeroAvatar + HeroName + HeroTagline + HeroSocialLinks + HeroActions

These five components live next to each other in `showcase-p2.jsx` in a three-column grid. We split them so each file is small and focused. Code below is for all six in one task — commit them together.

**Files:**
- Create: `frontend/src/lib/components/profile/HeroIdentity.svelte`
- Create: `frontend/src/lib/components/profile/HeroAvatar.svelte`
- Create: `frontend/src/lib/components/profile/HeroName.svelte`
- Create: `frontend/src/lib/components/profile/HeroTagline.svelte`
- Create: `frontend/src/lib/components/profile/HeroSocialLinks.svelte`
- Create: `frontend/src/lib/components/profile/HeroActions.svelte`

- [ ] **Step 1: HeroIdentity.svelte**

```svelte
<script lang="ts">
  import type { PublicProfile } from '$lib/api';
  import HeroAvatar from './HeroAvatar.svelte';
  import HeroName from './HeroName.svelte';
  import HeroTagline from './HeroTagline.svelte';
  import HeroSocialLinks from './HeroSocialLinks.svelte';
  import HeroActions from './HeroActions.svelte';

  let {
    profile,
    isOwner,
    onEditProfile
  }: {
    profile: PublicProfile;
    isOwner: boolean;
    onEditProfile: () => void;
  } = $props();
</script>

<section class="identity">
  <HeroAvatar handle={profile.handle} displayName={profile.display_name} />
  <div class="middle">
    <HeroName displayName={profile.display_name} />
    <HeroTagline tagline={profile.tagline} {isOwner} {onEditProfile} />
    <HeroSocialLinks links={profile.social_links} />
  </div>
  <HeroActions
    targetUserId={profile.id}
    {isOwner}
    {onEditProfile}
  />
</section>

<style>
  .identity {
    display: grid;
    grid-template-columns: 144px 1fr auto;
    gap: 24px;
    align-items: start;
    padding: 0 32px 24px;
    margin-top: -80px;
  }
  .middle { padding-top: 88px; }
  @media (max-width: 640px) {
    .identity {
      grid-template-columns: 1fr;
      margin-top: 16px;
      gap: 12px;
    }
    .middle { padding-top: 0; }
  }
</style>
```

- [ ] **Step 2: HeroAvatar.svelte** — 144×144 SQUARE (per spec; not a circle), `marginTop: -80` to overlap the cover, 4 px solid `--bg-canvas` border. If the user has uploaded an avatar (P2 doesn't add avatar upload — that's deferred), render the image; otherwise render initials in `--accent`/`--accent-ink`.

```svelte
<script lang="ts">
  let {
    handle,
    displayName
  }: {
    handle: string;
    displayName: string;
  } = $props();

  let initial = $derived((displayName[0] ?? handle[0] ?? 'U').toUpperCase());
</script>

<div class="avatar" aria-hidden="true">{initial}</div>

<style>
  .avatar {
    width: 144px;
    height: 144px;
    background: var(--accent);
    color: var(--accent-ink);
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-display);
    font-size: 64px;
    border: 4px solid var(--bg-canvas);
  }
  @media (max-width: 640px) {
    .avatar {
      width: 96px;
      height: 96px;
      font-size: 44px;
    }
  }
</style>
```

- [ ] **Step 3: HeroName.svelte** — Source Serif 4, italic on the *surname* (every word after the first).

```svelte
<script lang="ts">
  let { displayName }: { displayName: string } = $props();

  let parts = $derived(displayName.trim().split(/\s+/));
  let first = $derived(parts[0] ?? displayName);
  let rest = $derived(parts.slice(1).join(' '));
</script>

<h1 class="name">
  {first}{#if rest} <em>{rest}</em>{/if}
</h1>

<style>
  .name {
    font-family: var(--font-display, 'Source Serif 4', serif);
    font-weight: 400;
    font-size: 64px;
    line-height: 1;
    margin: 0;
  }
  .name em {
    font-style: italic;
  }
  @media (max-width: 640px) {
    .name { font-size: 40px; }
  }
</style>
```

- [ ] **Step 4: HeroTagline.svelte**

```svelte
<script lang="ts">
  let {
    tagline,
    isOwner,
    onEditProfile
  }: {
    tagline: string | null | undefined;
    isOwner: boolean;
    onEditProfile: () => void;
  } = $props();
</script>

{#if tagline}
  <p class="tag">{tagline}</p>
{:else if isOwner}
  <button type="button" class="tag-prompt" onclick={onEditProfile}>Add a tagline</button>
{/if}

<style>
  .tag {
    margin: 8px 0 0;
    font-size: 16px;
    color: var(--fg-secondary);
    max-width: 640px;
  }
  .tag-prompt {
    background: transparent;
    color: var(--accent);
    border: 0;
    padding: 4px 0;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
</style>
```

- [ ] **Step 5: HeroSocialLinks.svelte**

```svelte
<script lang="ts">
  import type { SocialLink } from '$lib/api';

  let { links }: { links: SocialLink[] } = $props();

  function label(p: SocialLink['platform']): string {
    switch (p) {
      case 'twitter':   return '𝕏';
      case 'instagram': return 'IG';
      case 'bluesky':   return 'BS';
      case 'astrobin':  return 'AB';
      case 'mastodon':  return 'MS';
      case 'youtube':   return 'YT';
      case 'website':   return '🌐';
    }
  }
</script>

{#if links.length > 0}
  <ul class="socials" aria-label="Social links">
    {#each links as link}
      <li>
        <a href={link.url} target="_blank" rel="noopener nofollow">{label(link.platform)}</a>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .socials {
    list-style: none;
    padding: 0;
    margin: 12px 0 0;
    display: flex;
    gap: 16px;
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .socials a { color: var(--fg-secondary); text-decoration: none; }
  .socials a:hover { color: var(--accent); }
</style>
```

- [ ] **Step 6: HeroActions.svelte**

```svelte
<script lang="ts">
  import FollowButton from '$lib/components/FollowButton.svelte';

  let {
    targetUserId,
    isOwner,
    onEditProfile
  }: {
    targetUserId: string;
    isOwner: boolean;
    onEditProfile: () => void;
  } = $props();
</script>

<div class="actions">
  {#if isOwner}
    <button type="button" class="btn-primary" onclick={onEditProfile}>Edit profile</button>
  {:else}
    <FollowButton userId={targetUserId} initialFollowing={false} />
  {/if}
</div>

<style>
  .actions {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .btn-primary {
    background: var(--accent);
    color: var(--accent-ink);
    border: 0;
    padding: 10px 16px;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
</style>
```

(`FollowButton` already exists from P1. If `initialFollowing` needs to come from server data, the orchestrator can pass it through — we're keeping the prop interface narrow; deeper wiring lands in Task 30.)

- [ ] **Step 7: Skip commit.**

---

### Task 19: HeroAbout + HeroEquipmentStrip + HeroLocationBadge

Three small read-side components — group them in one task.

**Files:**
- Create: `frontend/src/lib/components/profile/HeroAbout.svelte`
- Create: `frontend/src/lib/components/profile/HeroEquipmentStrip.svelte`
- Create: `frontend/src/lib/components/profile/HeroLocationBadge.svelte`

- [ ] **Step 1: HeroAbout.svelte** — renders sanitised `bio_html`. Collapses past N lines (use a CSS line-clamp on the *visitor* view; owner-side opens straight to the editor when clicked). When empty for an owner, render the empty-state copy "Tell visitors about your astrophotography"; when empty for a visitor, hide the section entirely.

```svelte
<script lang="ts">
  let {
    bio,
    isOwner,
    onEditProfile
  }: {
    bio: string | null | undefined;
    isOwner: boolean;
    onEditProfile: () => void;
  } = $props();

  let expanded = $state(false);
</script>

{#if bio}
  <section class="about">
    <h2 class="about-label">ABOUT</h2>
    <!-- bio_html is server-sanitised — this {@html} is intentional. -->
    <div class="bio" class:clamped={!expanded}>
      {@html bio}
    </div>
    <button type="button" class="more" onclick={() => (expanded = !expanded)}>
      {expanded ? 'less ↑' : 'more ↓'}
    </button>
  </section>
{:else if isOwner}
  <section class="about empty">
    <button type="button" class="prompt" onclick={onEditProfile}>
      Tell visitors about your astrophotography
    </button>
  </section>
{/if}

<style>
  .about {
    padding: 24px 32px;
    border-top: 1px solid var(--border-subtle);
  }
  .about-label {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
    margin: 0 0 8px;
    letter-spacing: 0.06em;
  }
  .bio {
    color: var(--fg-secondary);
    max-width: 640px;
    line-height: 1.55;
  }
  .bio.clamped {
    display: -webkit-box;
    -webkit-line-clamp: 4;
    line-clamp: 4;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .more {
    margin-top: 8px;
    background: none;
    border: 0;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: 11px;
    cursor: pointer;
    padding: 0;
  }
  .empty .prompt {
    background: transparent;
    color: var(--accent);
    border: 1px dashed var(--border-subtle);
    padding: 16px 20px;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
    width: 100%;
  }
</style>
```

- [ ] **Step 2: HeroEquipmentStrip.svelte** — five mono cells (SCOPE/CAM/MOUNT/FILTERS/GUIDING); cells with no value are hidden, not stubbed. When all are empty for an owner: show the prompt. For a visitor with all empty: hide.

```svelte
<script lang="ts">
  import type { EquipmentSummary } from '$lib/api';

  let {
    equipment,
    isOwner,
    onEditProfile
  }: {
    equipment: EquipmentSummary;
    isOwner: boolean;
    onEditProfile: () => void;
  } = $props();

  let cells = $derived(
    [
      ['SCOPE',  equipment.telescope],
      ['CAM',    equipment.camera],
      ['MOUNT',  equipment.mount],
      ['FILTERS', equipment.filters],
      ['GUIDING', equipment.guiding]
    ].filter(([, v]) => v != null && v.trim() !== '') as [string, string][]
  );
</script>

{#if cells.length > 0}
  <section class="strip">
    {#each cells as [label, value]}
      <div class="cell"><span class="lab">{label}</span> &nbsp; {value}</div>
    {/each}
  </section>
{:else if isOwner}
  <section class="strip empty">
    <button type="button" class="prompt" onclick={onEditProfile}>
      Add the gear behind your shots
    </button>
  </section>
{/if}

<style>
  .strip {
    padding: 16px 32px;
    border-top: 1px solid var(--border-subtle);
    display: flex;
    flex-wrap: wrap;
    gap: 24px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-secondary);
  }
  .lab { color: var(--fg-muted); }
  .empty .prompt {
    background: transparent;
    color: var(--accent);
    border: 1px dashed var(--border-subtle);
    padding: 12px 16px;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
</style>
```

- [ ] **Step 3: HeroLocationBadge.svelte**

```svelte
<script lang="ts">
  import type { LocationSummary } from '$lib/api';

  let {
    location,
    isOwner,
    onEditProfile
  }: {
    location: LocationSummary;
    isOwner: boolean;
    onEditProfile: () => void;
  } = $props();

  let parts = $derived(
    [
      location.location_text,
      location.bortle_class != null ? `Bortle ${location.bortle_class}` : null,
      location.sqm != null ? `SQM ${location.sqm.toFixed(2)}` : null
    ].filter(Boolean) as string[]
  );
</script>

{#if parts.length > 0}
  <section class="badge">
    {#each parts as p, i}
      <span>{p}</span>
      {#if i < parts.length - 1}<span class="dot">·</span>{/if}
    {/each}
  </section>
{:else if isOwner}
  <section class="badge empty">
    <button type="button" class="prompt" onclick={onEditProfile}>
      Where do you observe from?
    </button>
  </section>
{/if}

<style>
  .badge {
    padding: 12px 32px;
    border-top: 1px solid var(--border-subtle);
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-secondary);
    display: flex;
    gap: 8px;
  }
  .badge .dot { color: var(--fg-muted); }
  .empty .prompt {
    background: transparent;
    color: var(--accent);
    border: 1px dashed var(--border-subtle);
    padding: 8px 16px;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
</style>
```

- [ ] **Step 4: Skip commit.**

---

### Task 20: HeroStatsRow

**Files:**
- Create: `frontend/src/lib/components/profile/HeroStatsRow.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script lang="ts">
  import type { HeroStats } from '$lib/api';
  import { formatIntegration } from '$lib/format/integration';

  let { stats }: { stats: HeroStats } = $props();

  let cells = $derived([
    { num: stats.frames.toLocaleString(),                  label: 'frames' },
    { num: formatIntegration(stats.integration_seconds),   label: 'integration' },
    { num: stats.followers.toLocaleString(),               label: 'followers' },
    { num: stats.appreciations.toLocaleString(),           label: 'appreciations', accent: true },
    { num: stats.targets.toLocaleString(),                 label: 'targets shot' }
  ]);
</script>

<section class="row">
  {#each cells as c}
    <div class="cell" class:accent={c.accent}>
      <span class="num">{c.num}</span>
      <span class="lab">{c.label}</span>
    </div>
  {/each}
  <div class="member">Member since {stats.member_since_year}</div>
</section>

<style>
  .row {
    padding: 16px 32px;
    border-top: 1px solid var(--border-subtle);
    font-family: var(--font-mono);
    font-size: 12px;
    display: flex;
    flex-wrap: wrap;
    gap: 24px;
    align-items: baseline;
    color: var(--fg-secondary);
  }
  .cell { display: flex; gap: 6px; align-items: baseline; }
  .num { color: var(--fg-primary); font-size: 16px; }
  .lab { color: var(--fg-muted); }
  .accent .num { color: var(--accent); }
  .member { margin-left: auto; color: var(--fg-muted); }
</style>
```

- [ ] **Step 2: Skip commit.**

---

### Task 21: FeaturedRow + FeaturedTile

Owner sees 6 slots; empty slots render placeholder mono labels (`SLOT 01`..`SLOT 06`), and the first carries a `[+ Pin a photo]` button in `--accent`. Visitor sees only filled tiles, hidden if zero. Aspect 3:4 portrait.

**Files:**
- Create: `frontend/src/lib/components/profile/FeaturedRow.svelte`
- Create: `frontend/src/lib/components/profile/FeaturedTile.svelte`

- [ ] **Step 1: FeaturedTile.svelte**

```svelte
<script lang="ts">
  import type { FeaturedPhotoSummary } from '$lib/api';
  import Img from '$lib/components/Img.svelte';

  let {
    item,
    handle
  }: {
    item: FeaturedPhotoSummary;
    handle: string;
  } = $props();
</script>

<a class="tile" href="/u/{handle}/p/{item.short_id}" aria-label={item.target ?? 'Featured photo'}>
  <span class="rank">#{String(item.featured_position).padStart(2, '0')}</span>
  <Img
    photoId={item.id}
    w={600}
    aspectRatio="3/4"
    alt={item.target ?? 'Featured photo'}
    class="img"
  />
  <span class="cap">
    <span class="target">{item.target ?? 'Untitled'}</span>
    <span class="apps">{item.appreciations_count} ❤</span>
  </span>
</a>

<style>
  .tile {
    position: relative;
    display: block;
    aspect-ratio: 3 / 4;
    overflow: hidden;
    background: var(--bg-elevated);
  }
  .tile :global(.img) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .rank {
    position: absolute;
    top: 8px;
    left: 8px;
    z-index: 1;
    color: #fff;
    font-family: var(--font-mono);
    font-size: 11px;
    background: rgba(0, 0, 0, 0.45);
    padding: 2px 6px;
  }
  .cap {
    position: absolute;
    inset: auto 0 0 0;
    padding: 8px;
    background: linear-gradient(transparent, rgba(0, 0, 0, 0.6));
    color: #fff;
    font-family: var(--font-mono);
    font-size: 11px;
    display: flex;
    justify-content: space-between;
  }
</style>
```

- [ ] **Step 2: FeaturedRow.svelte**

```svelte
<script lang="ts">
  import type { FeaturedPhotoSummary } from '$lib/api';
  import FeaturedTile from './FeaturedTile.svelte';

  let {
    items,
    handle,
    isOwner,
    onPin
  }: {
    items: FeaturedPhotoSummary[];
    handle: string;
    isOwner: boolean;
    onPin: () => void;
  } = $props();

  let placeholders = $derived(isOwner ? Array.from({ length: 6 - items.length }, (_, i) => i) : []);
</script>

{#if items.length > 0 || isOwner}
  <section class="row" aria-label="Featured photos">
    {#each items as item (item.id)}
      <FeaturedTile {item} {handle} />
    {/each}
    {#each placeholders as i}
      <div class="slot">
        <span class="lab">SLOT {String(items.length + i + 1).padStart(2, '0')}</span>
        {#if i === 0}
          <button type="button" class="pin" onclick={onPin}>+ Pin a photo</button>
        {/if}
      </div>
    {/each}
  </section>
{/if}

<style>
  .row {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 8px;
    padding: 16px 32px;
    border-top: 1px solid var(--border-subtle);
  }
  .slot {
    aspect-ratio: 3 / 4;
    border: 1px dashed var(--border-subtle);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--fg-muted);
    font-family: var(--font-mono);
    font-size: 11px;
  }
  .pin {
    background: transparent;
    color: var(--accent);
    border: 1px solid var(--accent);
    padding: 6px 10px;
    font-family: var(--font-mono);
    font-size: 11px;
    cursor: pointer;
  }
  @media (max-width: 640px) {
    .row { grid-template-columns: repeat(2, 1fr); }
  }
</style>
```

- [ ] **Step 3: Skip commit.**

---

### Task 22: GalleryToolbar (sort selector)

**Files:**
- Create: `frontend/src/lib/components/profile/GalleryToolbar.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script lang="ts">
  type Sort = 'newest' | 'popular';

  let {
    sort = $bindable<Sort>('newest')
  }: {
    sort?: Sort;
  } = $props();
</script>

<header class="toolbar">
  <h2 class="title">Frames</h2>
  <label class="sort">
    <span class="lab">SORT</span>
    <select bind:value={sort}>
      <option value="newest">Newest</option>
      <option value="popular">Popular</option>
    </select>
  </label>
</header>

<style>
  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: 24px 32px 12px;
    border-top: 1px solid var(--border-subtle);
  }
  .title {
    font-family: var(--font-display, serif);
    font-size: 28px;
    font-weight: 400;
    margin: 0;
  }
  .sort {
    display: flex;
    gap: 8px;
    align-items: center;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-muted);
  }
  .sort select {
    background: transparent;
    color: var(--fg-primary);
    border: 1px solid var(--border-subtle);
    padding: 4px 8px;
    font-family: var(--font-mono);
    font-size: 12px;
  }
</style>
```

- [ ] **Step 2: Skip commit.**

---

### Task 23: PhotoGrid + PhotoTile (justified-rows)

`justified-layout` (Flickr) was added to `package.json` in P1. Use it to compute the layout. Each tile renders via `<Img>` with the blurhash placeholder.

**Files:**
- Create: `frontend/src/lib/components/profile/PhotoGrid.svelte`
- Create: `frontend/src/lib/components/profile/PhotoTile.svelte`

- [ ] **Step 1: PhotoTile.svelte**

```svelte
<script lang="ts">
  import type { GalleryPhoto } from '$lib/api';
  import Img from '$lib/components/Img.svelte';
  import PhotoTitle from '$lib/components/photos/PhotoTitle.svelte';

  let {
    photo,
    handle,
    width,
    height,
    top,
    left
  }: {
    photo: GalleryPhoto;
    handle: string;
    width: number;
    height: number;
    top: number;
    left: number;
  } = $props();
</script>

<a
  class="tile"
  style="width:{width}px; height:{height}px; transform: translate({left}px, {top}px);"
  href="/u/{handle}/p/{photo.short_id}"
  aria-label={photo.target ?? 'Untitled'}
>
  <Img
    photoId={photo.id}
    w={Math.round(width * 2)}
    alt={photo.target ?? 'Untitled'}
    blurhash={photo.blurhash ?? null}
    class="img"
  />
  <span class="cap">
    <PhotoTitle photo={{ target: photo.target }} size="md" />
    <span class="apps">{photo.appreciations_count} ❤</span>
  </span>
</a>

<style>
  .tile {
    position: absolute;
    top: 0;
    left: 0;
    overflow: hidden;
    background: var(--bg-elevated);
    transform-origin: top left;
  }
  .tile :global(.img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .cap {
    position: absolute;
    inset: auto 0 0 0;
    padding: 8px;
    background: linear-gradient(transparent, rgba(0, 0, 0, 0.55));
    color: #fff;
    font-family: var(--font-mono);
    font-size: 11px;
    display: flex;
    justify-content: space-between;
    opacity: 0;
    transition: opacity 0.15s ease-out;
  }
  .tile:hover .cap,
  .tile:focus-visible .cap {
    opacity: 1;
  }
</style>
```

- [ ] **Step 2: PhotoGrid.svelte**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import justifiedLayout from 'justified-layout';
  import type { GalleryPage, GalleryPhoto } from '$lib/api';
  import { fetchPhotosFeed } from '$lib/api/profile';
  import PhotoTile from './PhotoTile.svelte';

  let {
    handle,
    initial = null,
    sort = 'newest'
  }: {
    handle: string;
    initial?: GalleryPage | null;
    sort?: 'newest' | 'popular';
  } = $props();

  let photos = $state<GalleryPhoto[]>(initial?.photos ?? []);
  let nextCursor = $state<string | null>(initial?.next_cursor ?? null);
  let loading = $state(false);
  let containerWidth = $state(0);
  let containerEl: HTMLDivElement | null = null;

  // Re-fetch from scratch when sort changes.
  $effect(() => {
    const _ = sort; // dependency
    photos = [];
    nextCursor = null;
    loading = false;
    void loadMore();
  });

  async function loadMore() {
    if (loading) return;
    loading = true;
    try {
      const page = await fetchPhotosFeed(fetch, handle, {
        sort,
        cursor: nextCursor ?? undefined,
        limit: 24
      });
      photos = [...photos, ...page.photos];
      nextCursor = page.next_cursor ?? null;
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    if (containerEl) {
      containerWidth = containerEl.getBoundingClientRect().width;
      const ro = new ResizeObserver((entries) => {
        for (const e of entries) containerWidth = e.contentRect.width;
      });
      ro.observe(containerEl);
      return () => ro.disconnect();
    }
  });

  let layout = $derived.by(() => {
    if (containerWidth <= 0 || photos.length === 0) {
      return { containerHeight: 0, boxes: [] };
    }
    const isMobile = containerWidth < 640;
    const aspectRatios = photos.map((p) => {
      const w = p.width ?? 3;
      const h = p.height ?? 2;
      return Math.max(0.2, Math.min(5, w / h));
    });
    const result = justifiedLayout(aspectRatios, {
      containerWidth,
      containerPadding: 0,
      boxSpacing: 8,
      targetRowHeight: isMobile ? 140 : 220
    });
    return result;
  });
</script>

<div class="grid" bind:this={containerEl} style="height:{layout.containerHeight}px">
  {#each photos as photo, i (photo.id)}
    {#if layout.boxes[i]}
      <PhotoTile
        {photo}
        {handle}
        width={layout.boxes[i].width}
        height={layout.boxes[i].height}
        top={layout.boxes[i].top}
        left={layout.boxes[i].left}
      />
    {/if}
  {/each}
</div>

{#if nextCursor}
  <div class="more">
    <button type="button" class="btn-more" disabled={loading} onclick={() => void loadMore()}>
      {loading ? 'Loading…' : 'Load more'}
    </button>
  </div>
{:else if photos.length === 0 && !loading}
  <p class="empty">No photos yet.</p>
{/if}

<style>
  .grid {
    position: relative;
    margin: 0 32px;
  }
  .more {
    display: flex;
    justify-content: center;
    padding: 24px;
  }
  .btn-more {
    background: transparent;
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 8px 16px;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
  .empty {
    padding: 48px 32px;
    color: var(--fg-muted);
    font-family: var(--font-mono);
    font-size: 12px;
  }
</style>
```

- [ ] **Step 3: Skip commit.**

---

## Frontend — profile editor

Tiptap-backed modal. Save-on-blur per section. The modal sits inside the viewmode==='owner' branch of `+page.svelte` and is closed by default; opening is wired in Task 30.

### Task 24: tiptapAllowlist.ts + drift unit test

**Files:**
- Create: `frontend/src/lib/components/profile/editor/tiptapAllowlist.ts`
- Create: `frontend/src/lib/components/profile/editor/tiptapAllowlist.test.ts`

- [ ] **Step 1: Write the failing test**

```ts
import { describe, it, expect } from 'vitest';
import { ALLOWED_HTML_TAGS } from './tiptapAllowlist';
import shared from '../../../../../../backend/data/bio-allowed-tags.json';

describe('tiptapAllowlist', () => {
  it('matches the shared backend JSON', () => {
    expect([...ALLOWED_HTML_TAGS].sort()).toEqual([...shared.tags].sort());
  });
});
```

The relative path goes up out of `frontend/src/lib/components/profile/editor/` into the worktree root and back down to `backend/data/`. Adjust the dot count if your tsconfig disallows reaching across packages — in that case, write the same constants locally and have a Rust-side test pull them via a script. This plan assumes Vitest can resolve cross-package JSON via `resolveJsonModule: true` (which is on by default in SvelteKit).

- [ ] **Step 2: Run — fails (file doesn't exist)**

```
cd frontend && pnpm vitest run src/lib/components/profile/editor/tiptapAllowlist.test.ts
```

- [ ] **Step 3: Write the file**

```ts
/**
 * Allowed HTML tags for the bio editor. MUST match
 * `backend/data/bio-allowed-tags.json` exactly — drift is verified by
 * `tiptapAllowlist.test.ts`.
 */
export const ALLOWED_HTML_TAGS = [
  'a',
  'blockquote',
  'br',
  'code',
  'em',
  'h2',
  'h3',
  'h4',
  'li',
  'ol',
  'p',
  'strong',
  'u',
  'ul'
] as const;

export type AllowedTag = (typeof ALLOWED_HTML_TAGS)[number];
```

- [ ] **Step 4: Run — passes**

```
cd frontend && pnpm vitest run src/lib/components/profile/editor/tiptapAllowlist.test.ts
```

- [ ] **Step 5: Commit**

```
git add frontend/src/lib/components/profile/editor/tiptapAllowlist.ts frontend/src/lib/components/profile/editor/tiptapAllowlist.test.ts
git commit -m "feat(editor): tiptap allowlist mirrors backend bio sanitiser"
```

---

### Task 25: AboutSection.svelte (Tiptap editor configured to the allowlist)

**Files:**
- Create: `frontend/src/lib/components/profile/editor/AboutSection.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Editor } from '@tiptap/core';
  import StarterKit from '@tiptap/starter-kit';
  import Link from '@tiptap/extension-link';

  let {
    initial = '',
    onSave
  }: {
    initial?: string;
    onSave: (html: string) => Promise<void> | void;
  } = $props();

  let el: HTMLDivElement | null = $state(null);
  let editor: Editor | null = null;
  let dirty = $state(false);
  let saving = $state(false);

  onMount(() => {
    if (!el) return;
    editor = new Editor({
      element: el,
      extensions: [
        StarterKit.configure({
          // Allowlist: p, br, strong, em, u, h2, h3, h4, ul, ol, li, blockquote, code, a.
          // Disable starter-kit nodes/marks that emit non-allowed HTML.
          codeBlock: false,
          horizontalRule: false,
          heading: { levels: [2, 3, 4] }
        }),
        Link.configure({
          openOnClick: false,
          HTMLAttributes: { rel: 'nofollow noopener', target: '_blank' },
          protocols: ['http', 'https', 'mailto']
        })
      ],
      content: initial,
      editorProps: {
        attributes: {
          class: 'tiptap-bio'
        }
      },
      onUpdate: () => {
        dirty = true;
      }
    });
    return () => {
      editor?.destroy();
      editor = null;
    };
  });

  async function handleBlur() {
    if (!editor || !dirty || saving) return;
    saving = true;
    try {
      const html = editor.getHTML();
      await onSave(html);
      dirty = false;
    } finally {
      saving = false;
    }
  }

  function toggle(cmd: () => void) {
    return () => {
      cmd();
      el?.querySelector<HTMLElement>('.tiptap-bio')?.focus();
    };
  }
</script>

<section class="about-editor">
  <div class="toolbar" role="toolbar" aria-label="Bio formatting">
    <button type="button" onclick={toggle(() => editor?.chain().focus().toggleBold().run())}>B</button>
    <button type="button" onclick={toggle(() => editor?.chain().focus().toggleItalic().run())}><em>I</em></button>
    <button type="button" onclick={toggle(() => editor?.chain().focus().toggleUnderline().run())}><u>U</u></button>
    <button type="button" onclick={toggle(() => editor?.chain().focus().toggleHeading({ level: 2 }).run())}>H₂</button>
    <button type="button" onclick={toggle(() => editor?.chain().focus().toggleHeading({ level: 3 }).run())}>H₃</button>
    <button type="button" onclick={toggle(() => editor?.chain().focus().toggleBulletList().run())}>•</button>
    <button type="button" onclick={toggle(() => editor?.chain().focus().toggleOrderedList().run())}>1.</button>
    <button type="button" onclick={toggle(() => editor?.chain().focus().toggleBlockquote().run())}>"</button>
    <button type="button" onclick={toggle(() => editor?.chain().focus().toggleCode().run())}>&lt;&gt;</button>
    <button type="button" onclick={() => {
      const url = prompt('Link URL:');
      if (url) editor?.chain().focus().setLink({ href: url }).run();
    }}>🔗</button>
  </div>
  <div bind:this={el} onblur={handleBlur} class="editor-host"></div>
  {#if saving}<span class="saving">Saving…</span>{/if}
</section>

<style>
  .about-editor {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .toolbar {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    border: 1px solid var(--border-subtle);
    border-bottom: 0;
    padding: 4px;
    background: var(--bg-elevated);
  }
  .toolbar button {
    background: transparent;
    border: 0;
    color: var(--fg-primary);
    padding: 4px 8px;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
  .toolbar button:hover {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .editor-host {
    border: 1px solid var(--border-subtle);
    background: var(--bg-canvas);
    min-height: 160px;
    padding: 12px;
  }
  .editor-host :global(.tiptap-bio) {
    outline: none;
    color: var(--fg-primary);
    font-family: inherit;
    line-height: 1.55;
  }
  .saving {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
  }
</style>
```

The `<u>` mark needs an underline extension. Starter-kit doesn't include it by default. Add it now:

```
cd frontend && pnpm add @tiptap/extension-underline
```

Then add to the imports + extensions array in this component:

```ts
import Underline from '@tiptap/extension-underline';
// inside extensions:
Underline,
```

(Tiptap's `Underline` mark renders as `<u>`. If for some reason your version emits `<span style="text-decoration:underline">` instead, that fails the sanitiser silently — verify by saving a sample with underline and running it through the GET response in DevTools.)

- [ ] **Step 2: Skip commit.**

---

### Task 26: IdentitySection + EquipmentSection + LocationSection + BortleLadder + SocialLinksSection

These five sections live in the editor modal as collapsing groups. Save-on-blur means: each section keeps a local `dirty` flag and a `commit()` that calls `patchOwnerProfile` with only that section's payload when the section loses focus.

**Files:**
- Create: `frontend/src/lib/components/profile/editor/IdentitySection.svelte`
- Create: `frontend/src/lib/components/profile/editor/EquipmentSection.svelte`
- Create: `frontend/src/lib/components/profile/editor/LocationSection.svelte`
- Create: `frontend/src/lib/components/profile/editor/BortleLadder.svelte`
- Create: `frontend/src/lib/components/profile/editor/SocialLinksSection.svelte`

- [ ] **Step 1: BortleLadder.svelte** — 9-cell segmented; accent on selected.

```svelte
<script lang="ts">
  let {
    value = $bindable<number | null>(null)
  }: {
    value?: number | null;
  } = $props();
</script>

<div class="ladder" role="radiogroup" aria-label="Bortle class">
  {#each Array.from({ length: 9 }, (_, i) => i + 1) as cell}
    <button
      type="button"
      role="radio"
      aria-checked={value === cell}
      class:selected={value === cell}
      onclick={() => (value = value === cell ? null : cell)}
    >
      {cell}
    </button>
  {/each}
</div>

<style>
  .ladder {
    display: grid;
    grid-template-columns: repeat(9, 1fr);
    gap: 0;
    border: 1px solid var(--border-subtle);
  }
  .ladder button {
    background: transparent;
    color: var(--fg-secondary);
    border: 0;
    border-right: 1px solid var(--border-subtle);
    padding: 8px 0;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
  .ladder button:last-child { border-right: 0; }
  .ladder .selected {
    background: var(--accent);
    color: var(--accent-ink);
  }
</style>
```

- [ ] **Step 2: IdentitySection.svelte**

```svelte
<script lang="ts">
  let {
    displayName = $bindable<string>(''),
    tagline = $bindable<string | null>(null),
    onCommit
  }: {
    displayName?: string;
    tagline?: string | null;
    onCommit: (patch: { display_name?: string; tagline?: string | null }) => Promise<void>;
  } = $props();

  let savedName = $state(displayName);
  let savedTag = $state(tagline);

  async function commitName() {
    if (displayName === savedName) return;
    await onCommit({ display_name: displayName });
    savedName = displayName;
  }
  async function commitTag() {
    const norm = tagline?.trim() === '' ? null : tagline;
    if (norm === savedTag) return;
    await onCommit({ tagline: norm });
    savedTag = norm;
  }
</script>

<fieldset class="section">
  <legend>Identity</legend>
  <label class="field">
    <span>Display name</span>
    <input type="text" bind:value={displayName} onblur={() => void commitName()} maxlength="60" />
  </label>
  <label class="field">
    <span>Tagline</span>
    <input type="text" bind:value={tagline} onblur={() => void commitTag()} maxlength="140" />
  </label>
</fieldset>

<style>
  .section {
    border: 1px solid var(--border-subtle);
    padding: 16px;
    margin: 0 0 16px;
  }
  legend {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-muted);
    padding: 0 6px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 12px;
  }
  .field span {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
  }
  .field input {
    background: var(--bg-canvas);
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 8px 10px;
    font-size: 14px;
  }
</style>
```

- [ ] **Step 3: EquipmentSection.svelte**

Five plain text inputs. Autocomplete suggestions can ride on the existing `/api/equipment/autocomplete` (P1) but the spec marks the "▾ suggest" affordance as P3 polish — for P2 we ship plain inputs and revisit. Save-on-blur calls `onCommit` with the equipment object whenever any cell loses focus and at least one cell changed.

```svelte
<script lang="ts">
  import type { EquipmentSummary } from '$lib/api';

  let {
    equipment = $bindable<EquipmentSummary>({
      telescope: null, camera: null, mount: null, filters: null, guiding: null
    }),
    onCommit
  }: {
    equipment?: EquipmentSummary;
    onCommit: (patch: { equipment: EquipmentSummary }) => Promise<void>;
  } = $props();

  let saved = $state(structuredClone(equipment));

  function changed(): boolean {
    return JSON.stringify(saved) !== JSON.stringify(equipment);
  }

  async function commit() {
    if (!changed()) return;
    await onCommit({ equipment: { ...equipment } });
    saved = structuredClone(equipment);
  }

  function norm(s: string | null): string | null {
    if (s == null) return null;
    const t = s.trim();
    return t === '' ? null : t;
  }
</script>

<fieldset class="section" onfocusout={() => void commit()}>
  <legend>Equipment</legend>
  {#each [
    ['Scope',   'telescope'],
    ['Camera',  'camera'],
    ['Mount',   'mount'],
    ['Filters', 'filters'],
    ['Guiding', 'guiding']
  ] as [label, key]}
    <label class="field">
      <span>{label}</span>
      <input
        type="text"
        value={equipment[key as keyof EquipmentSummary] ?? ''}
        oninput={(e) => {
          equipment = { ...equipment, [key]: norm((e.target as HTMLInputElement).value) };
        }}
      />
    </label>
  {/each}
</fieldset>

<style>
  .section {
    border: 1px solid var(--border-subtle);
    padding: 16px;
    margin: 0 0 16px;
  }
  legend { font-family: var(--font-mono); font-size: 12px; color: var(--fg-muted); padding: 0 6px; }
  .field { display: flex; flex-direction: column; gap: 6px; margin-bottom: 12px; }
  .field span { font-family: var(--font-mono); font-size: 11px; color: var(--fg-muted); }
  .field input {
    background: var(--bg-canvas);
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 8px 10px;
    font-size: 14px;
  }
</style>
```

- [ ] **Step 4: LocationSection.svelte**

```svelte
<script lang="ts">
  import type { LocationSummary } from '$lib/api';
  import BortleLadder from './BortleLadder.svelte';

  let {
    location = $bindable<LocationSummary>({ location_text: null, bortle_class: null, sqm: null }),
    onCommit
  }: {
    location?: LocationSummary;
    onCommit: (patch: { location: LocationSummary }) => Promise<void>;
  } = $props();

  let saved = $state(structuredClone(location));

  function changed(): boolean {
    return JSON.stringify(saved) !== JSON.stringify(location);
  }
  async function commit() {
    if (!changed()) return;
    await onCommit({ location: { ...location } });
    saved = structuredClone(location);
  }
</script>

<fieldset class="section" onfocusout={() => void commit()}>
  <legend>Location & sky</legend>
  <label class="field">
    <span>City / region</span>
    <input
      type="text"
      value={location.location_text ?? ''}
      oninput={(e) => {
        const v = (e.target as HTMLInputElement).value.trim();
        location = { ...location, location_text: v === '' ? null : v };
      }}
    />
  </label>
  <div class="field">
    <span>Bortle class</span>
    <BortleLadder
      bind:value={() => location.bortle_class, (v) => (location = { ...location, bortle_class: v })}
    />
  </div>
  <label class="field">
    <span>SQM (optional)</span>
    <input
      type="number"
      step="0.01"
      min="0"
      max="99.99"
      value={location.sqm ?? ''}
      oninput={(e) => {
        const v = (e.target as HTMLInputElement).valueAsNumber;
        location = { ...location, sqm: Number.isFinite(v) ? v : null };
      }}
    />
  </label>
</fieldset>

<style>
  .section { border: 1px solid var(--border-subtle); padding: 16px; margin: 0 0 16px; }
  legend { font-family: var(--font-mono); font-size: 12px; color: var(--fg-muted); padding: 0 6px; }
  .field { display: flex; flex-direction: column; gap: 6px; margin-bottom: 12px; }
  .field span { font-family: var(--font-mono); font-size: 11px; color: var(--fg-muted); }
  .field input {
    background: var(--bg-canvas);
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 8px 10px;
    font-size: 14px;
  }
</style>
```

(Note: the `bind:value` form on `<BortleLadder>` uses Svelte 5's getter/setter binding pair. If your Svelte version doesn't yet support that syntax, replace with `value={location.bortle_class}` + an `onchange` callback prop on `<BortleLadder>`.)

- [ ] **Step 5: SocialLinksSection.svelte**

```svelte
<script lang="ts">
  import type { SocialLink, SocialPlatform } from '$lib/api';

  let {
    links = $bindable<SocialLink[]>([]),
    onCommit
  }: {
    links?: SocialLink[];
    onCommit: (patch: { social_links: SocialLink[] }) => Promise<void>;
  } = $props();

  let saved = $state(structuredClone(links));
  const PLATFORMS: SocialPlatform[] = [
    'twitter', 'instagram', 'bluesky', 'astrobin', 'mastodon', 'youtube', 'website'
  ];

  function changed(): boolean {
    return JSON.stringify(saved) !== JSON.stringify(links);
  }
  async function commit() {
    if (!changed()) return;
    await onCommit({ social_links: links });
    saved = structuredClone(links);
  }

  function add() {
    if (links.length >= 6) return;
    const used = new Set(links.map((l) => l.platform));
    const next = PLATFORMS.find((p) => !used.has(p)) ?? 'website';
    links = [...links, { platform: next, url: '' }];
  }
  function remove(i: number) {
    links = links.filter((_, idx) => idx !== i);
  }
</script>

<fieldset class="section" onfocusout={() => void commit()}>
  <legend>Social links</legend>
  {#each links as link, i (i)}
    <div class="row">
      <select
        value={link.platform}
        onchange={(e) => {
          links = links.map((l, idx) =>
            idx === i ? { ...l, platform: (e.target as HTMLSelectElement).value as SocialPlatform } : l
          );
        }}
      >
        {#each PLATFORMS as p}
          <option value={p}>{p}</option>
        {/each}
      </select>
      <input
        type="url"
        placeholder="https://…"
        value={link.url}
        oninput={(e) => {
          links = links.map((l, idx) =>
            idx === i ? { ...l, url: (e.target as HTMLInputElement).value } : l
          );
        }}
      />
      <button type="button" class="remove" aria-label="Remove" onclick={() => remove(i)}>×</button>
    </div>
  {/each}
  {#if links.length < 6}
    <button type="button" class="add" onclick={add}>+ Add link</button>
  {/if}
</fieldset>

<style>
  .section { border: 1px solid var(--border-subtle); padding: 16px; margin: 0 0 16px; }
  legend { font-family: var(--font-mono); font-size: 12px; color: var(--fg-muted); padding: 0 6px; }
  .row {
    display: grid;
    grid-template-columns: 140px 1fr 32px;
    gap: 8px;
    margin-bottom: 8px;
  }
  .row input,
  .row select {
    background: var(--bg-canvas);
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 6px 8px;
    font-size: 13px;
  }
  .remove {
    background: transparent;
    color: var(--fg-muted);
    border: 1px solid var(--border-subtle);
    cursor: pointer;
  }
  .add {
    background: transparent;
    color: var(--accent);
    border: 0;
    padding: 6px 0;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
</style>
```

- [ ] **Step 6: Skip commit.**

---

### Task 27: ProfileEditor.svelte — modal shell

**Files:**
- Create: `frontend/src/lib/components/profile/editor/ProfileEditor.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script lang="ts">
  import { fetchOwnerProfile, patchOwnerProfile } from '$lib/api/profile';
  import type { Profile } from '$lib/api';
  import IdentitySection from './IdentitySection.svelte';
  import AboutSection from './AboutSection.svelte';
  import EquipmentSection from './EquipmentSection.svelte';
  import LocationSection from './LocationSection.svelte';
  import SocialLinksSection from './SocialLinksSection.svelte';

  let {
    open = $bindable<boolean>(false),
    onSaved = () => {}
  }: {
    open?: boolean;
    onSaved?: (profile: Profile) => void;
  } = $props();

  let profile = $state<Profile | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);

  $effect(() => {
    if (open && !profile && !loading) {
      void load();
    }
  });

  async function load() {
    loading = true;
    error = null;
    try {
      profile = await fetchOwnerProfile(fetch);
    } catch (e) {
      error = (e as Error).message;
    } finally {
      loading = false;
    }
  }

  async function commit(patch: Partial<Profile> & { social_links?: Profile['social_links'] }) {
    if (!profile) return;
    await patchOwnerProfile(fetch, patch);
    profile = { ...profile, ...patch };
    onSaved(profile);
  }

  function close() {
    open = false;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) close();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <div class="overlay" role="dialog" aria-modal="true" aria-label="Edit profile">
    <button type="button" class="scrim" aria-label="Close" onclick={close}></button>
    <div class="dialog">
      <header>
        <h2>Edit profile</h2>
        <button type="button" class="x" onclick={close} aria-label="Close">×</button>
      </header>
      {#if loading}
        <p class="status">Loading…</p>
      {:else if error}
        <p class="status err">{error}</p>
      {:else if profile}
        <IdentitySection
          bind:displayName={() => profile!.display_name, (v) => (profile = { ...profile!, display_name: v })}
          bind:tagline={() => profile!.tagline ?? null, (v) => (profile = { ...profile!, tagline: v })}
          onCommit={commit}
        />
        <AboutSection
          initial={profile.bio_html ?? ''}
          onSave={(html) => commit({ bio_html: html })}
        />
        <EquipmentSection
          bind:equipment={() => profile!.equipment, (v) => (profile = { ...profile!, equipment: v })}
          onCommit={commit}
        />
        <LocationSection
          bind:location={() => profile!.location, (v) => (profile = { ...profile!, location: v })}
          onCommit={commit}
        />
        <SocialLinksSection
          bind:links={() => profile!.social_links, (v) => (profile = { ...profile!, social_links: v })}
          onCommit={commit}
        />
      {/if}
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: stretch;
    justify-content: flex-end;
  }
  .scrim {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    border: 0;
    cursor: default;
  }
  .dialog {
    position: relative;
    width: 480px;
    max-width: 100vw;
    background: var(--bg-canvas);
    border-left: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    overflow-y: auto;
    padding: 16px;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-subtle);
    padding-bottom: 12px;
    margin-bottom: 16px;
  }
  header h2 { margin: 0; font-family: var(--font-display, serif); font-weight: 400; }
  .x {
    background: transparent;
    color: var(--fg-muted);
    border: 0;
    font-size: 24px;
    cursor: pointer;
  }
  .status { color: var(--fg-muted); font-family: var(--font-mono); font-size: 12px; }
  .status.err { color: var(--danger, #c33); }
</style>
```

- [ ] **Step 2: Skip commit** — bundle frontend in Task 30.

---

## Frontend — cover picker + featured controls

### Task 28: PhotoPickerGrid (shared between cover picker and featured pin)

A small grid of the user's published photos, click to select. `multi=false` picks one; `multi=true` reserved for future. Filters out a supplied `excludeIds` list.

**Files:**
- Create: `frontend/src/lib/components/profile/editor/PhotoPickerGrid.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchPhotosFeed } from '$lib/api/profile';
  import type { GalleryPhoto } from '$lib/api';
  import Img from '$lib/components/Img.svelte';

  let {
    handle,
    excludeIds = [],
    onPick
  }: {
    handle: string;
    excludeIds?: string[];
    onPick: (photo: GalleryPhoto) => void;
  } = $props();

  let photos = $state<GalleryPhoto[]>([]);
  let nextCursor = $state<string | null>(null);
  let loading = $state(false);

  let visible = $derived(photos.filter((p) => !excludeIds.includes(p.id)));

  onMount(() => void load());

  async function load() {
    if (loading) return;
    loading = true;
    try {
      const page = await fetchPhotosFeed(fetch, handle, {
        cursor: nextCursor ?? undefined,
        limit: 24
      });
      photos = [...photos, ...page.photos];
      nextCursor = page.next_cursor ?? null;
    } finally {
      loading = false;
    }
  }
</script>

<div class="picker">
  {#if visible.length === 0 && !loading}
    <p class="empty">No published photos to choose from yet.</p>
  {:else}
    <ul class="grid">
      {#each visible as p (p.id)}
        <li>
          <button type="button" class="cell" onclick={() => onPick(p)}>
            <Img photoId={p.id} w={300} aspectRatio="1/1" alt={p.target ?? 'Untitled'} class="img" />
            <span class="cap">{p.target ?? 'Untitled'}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
  {#if nextCursor}
    <button type="button" class="more" disabled={loading} onclick={() => void load()}>
      {loading ? 'Loading…' : 'Load more'}
    </button>
  {/if}
</div>

<style>
  .picker { display: flex; flex-direction: column; gap: 12px; }
  .grid {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
  }
  .cell {
    background: transparent;
    border: 1px solid var(--border-subtle);
    padding: 0;
    cursor: pointer;
    position: relative;
    aspect-ratio: 1 / 1;
    overflow: hidden;
  }
  .cell:hover { border-color: var(--accent); }
  .cell :global(.img) { width: 100%; height: 100%; object-fit: cover; display: block; }
  .cap {
    position: absolute;
    inset: auto 0 0 0;
    padding: 4px 6px;
    color: #fff;
    background: rgba(0, 0, 0, 0.55);
    font-family: var(--font-mono);
    font-size: 10px;
    text-align: left;
  }
  .empty { color: var(--fg-muted); font-family: var(--font-mono); font-size: 12px; }
  .more {
    align-self: center;
    background: transparent;
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 6px 12px;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
</style>
```

- [ ] **Step 2: Skip commit.**

---

### Task 29: CoverPickerModal.svelte

**Files:**
- Create: `frontend/src/lib/components/profile/editor/CoverPickerModal.svelte`

- [ ] **Step 1: Write the component**

```svelte
<script lang="ts">
  import type { GalleryPhoto } from '$lib/api';
  import { setCover } from '$lib/api/profile';
  import PhotoPickerGrid from './PhotoPickerGrid.svelte';

  let {
    open = $bindable<boolean>(false),
    handle,
    onPicked = () => {}
  }: {
    open?: boolean;
    handle: string;
    onPicked?: (photoId: string | null) => void;
  } = $props();

  function close() { open = false; }

  async function pick(p: GalleryPhoto) {
    await setCover(fetch, p.id);
    onPicked(p.id);
    close();
  }
  async function clear() {
    await setCover(fetch, null);
    onPicked(null);
    close();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) close();
  }
</script>

<svelte:window onkeydown={onKey} />

{#if open}
  <div class="overlay" role="dialog" aria-modal="true" aria-label="Pick a cover photo">
    <button type="button" class="scrim" aria-label="Close" onclick={close}></button>
    <div class="dialog">
      <header>
        <h2>Pick a cover</h2>
        <div class="actions">
          <button type="button" class="clear" onclick={() => void clear()}>Clear cover</button>
          <button type="button" class="x" onclick={close} aria-label="Close">×</button>
        </div>
      </header>
      <PhotoPickerGrid {handle} onPick={(p) => void pick(p)} />
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: grid;
    place-items: center;
  }
  .scrim {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    border: 0;
    cursor: default;
  }
  .dialog {
    position: relative;
    width: 720px;
    max-width: 95vw;
    max-height: 80vh;
    overflow-y: auto;
    background: var(--bg-canvas);
    border: 1px solid var(--border-subtle);
    padding: 16px;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-subtle);
    padding-bottom: 12px;
    margin-bottom: 16px;
  }
  header h2 { margin: 0; font-family: var(--font-display, serif); font-weight: 400; }
  .actions { display: flex; gap: 8px; align-items: center; }
  .clear {
    background: transparent;
    border: 1px solid var(--border-subtle);
    color: var(--fg-secondary);
    padding: 6px 10px;
    font-family: var(--font-mono);
    font-size: 11px;
    cursor: pointer;
  }
  .x { background: transparent; color: var(--fg-muted); border: 0; font-size: 24px; cursor: pointer; }
</style>
```

- [ ] **Step 2: Skip commit.**

---

### Task 30: Featured drag-reorder + pin / unpin controls

The owner-side `<FeaturedRow>` from Task 21 renders the items. P2 layers on:
- A pin picker triggered by `[+ Pin a photo]` (placeholder slot 1).
- Per-tile drag handle and unpin button on hover.
- Drag-and-drop reorder via `@neodrag/svelte`; persists on drop.

This is the most behaviour-heavy piece in the editor. Layer it onto the existing `FeaturedRow` rather than duplicating the component.

**Files:**
- Modify: `frontend/src/lib/components/profile/FeaturedRow.svelte`
- Modify: `frontend/src/lib/components/profile/FeaturedTile.svelte`
- Create: `frontend/src/lib/components/profile/editor/FeaturedPinModal.svelte`
- Create: `frontend/src/lib/components/profile/editor/FeaturedDraggableTile.svelte`

- [ ] **Step 1: FeaturedPinModal.svelte** — opens the picker filtered by already-pinned ids; calls `pinFeatured` and informs the parent.

```svelte
<script lang="ts">
  import { pinFeatured } from '$lib/api/profile';
  import type { GalleryPhoto } from '$lib/api';
  import PhotoPickerGrid from './PhotoPickerGrid.svelte';

  let {
    open = $bindable<boolean>(false),
    handle,
    excludeIds = [],
    onPinned = () => {}
  }: {
    open?: boolean;
    handle: string;
    excludeIds?: string[];
    onPinned?: (photoId: string) => void;
  } = $props();

  function close() { open = false; }

  async function pick(p: GalleryPhoto) {
    await pinFeatured(fetch, p.id);
    onPinned(p.id);
    close();
  }
</script>

{#if open}
  <div class="overlay" role="dialog" aria-modal="true">
    <button type="button" class="scrim" onclick={close} aria-label="Close"></button>
    <div class="dialog">
      <header>
        <h2>Pin a photo</h2>
        <button type="button" class="x" onclick={close} aria-label="Close">×</button>
      </header>
      <PhotoPickerGrid {handle} {excludeIds} onPick={(p) => void pick(p)} />
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed; inset: 0; z-index: 100; display: grid; place-items: center;
  }
  .scrim {
    position: absolute; inset: 0; background: rgba(0, 0, 0, 0.55); border: 0; cursor: default;
  }
  .dialog {
    position: relative;
    width: 720px;
    max-width: 95vw;
    max-height: 80vh;
    overflow-y: auto;
    background: var(--bg-canvas);
    border: 1px solid var(--border-subtle);
    padding: 16px;
  }
  header {
    display: flex; justify-content: space-between; align-items: center;
    border-bottom: 1px solid var(--border-subtle);
    padding-bottom: 12px; margin-bottom: 16px;
  }
  header h2 { margin: 0; font-family: var(--font-display, serif); font-weight: 400; }
  .x { background: transparent; color: var(--fg-muted); border: 0; font-size: 24px; cursor: pointer; }
</style>
```

- [ ] **Step 2: FeaturedDraggableTile.svelte** — wraps `<FeaturedTile>` with drag handle and unpin button. Uses `@neodrag/svelte`'s action.

```svelte
<script lang="ts">
  import { draggable } from '@neodrag/svelte';
  import type { FeaturedPhotoSummary } from '$lib/api';
  import FeaturedTile from '../FeaturedTile.svelte';

  let {
    item,
    handle,
    onDragStart,
    onDragEnd,
    onUnpin
  }: {
    item: FeaturedPhotoSummary;
    handle: string;
    onDragStart: (id: string) => void;
    onDragEnd: (id: string, dx: number, dy: number) => void;
    onUnpin: (id: string) => void;
  } = $props();
</script>

<div
  class="wrap"
  use:draggable={{
    bounds: 'parent',
    onDragStart: () => onDragStart(item.id),
    onDragEnd: (e) => onDragEnd(item.id, e.offsetX, e.offsetY)
  }}
>
  <FeaturedTile {item} {handle} />
  <button type="button" class="unpin" aria-label="Unpin" onclick={() => onUnpin(item.id)}>✕</button>
</div>

<style>
  .wrap { position: relative; cursor: grab; }
  .wrap:active { cursor: grabbing; }
  .unpin {
    position: absolute;
    top: 6px;
    right: 6px;
    z-index: 2;
    background: rgba(0, 0, 0, 0.55);
    color: #fff;
    border: 0;
    width: 24px;
    height: 24px;
    cursor: pointer;
    font-size: 12px;
  }
</style>
```

- [ ] **Step 3: Modify `FeaturedRow.svelte` to delegate drag/unpin to a new editor mode**

Replace the body of `FeaturedRow.svelte` with the version below, which adds an `editorMode` prop and consumes the modal+draggable tiles when on. Visitors and the read-only owner view (mode `'visitor'` / `'owner'` without editor) keep the old behaviour.

```svelte
<script lang="ts">
  import type { FeaturedPhotoSummary } from '$lib/api';
  import { unpinFeatured, reorderFeatured } from '$lib/api/profile';
  import FeaturedTile from './FeaturedTile.svelte';
  import FeaturedDraggableTile from './editor/FeaturedDraggableTile.svelte';
  import FeaturedPinModal from './editor/FeaturedPinModal.svelte';

  let {
    items: incoming,
    handle,
    isOwner,
    editorMode = false
  }: {
    items: FeaturedPhotoSummary[];
    handle: string;
    isOwner: boolean;
    editorMode?: boolean;
  } = $props();

  let local = $state<FeaturedPhotoSummary[]>([...incoming]);
  $effect(() => {
    local = [...incoming];
  });

  let pinOpen = $state(false);

  let placeholders = $derived(isOwner ? Array.from({ length: 6 - local.length }, (_, i) => i) : []);

  // Compute tile pixel width — used to translate a drag distance into a slot delta.
  let containerEl: HTMLElement | null = null;
  let tileWidth = $state(0);
  $effect(() => {
    if (!containerEl) return;
    const ro = new ResizeObserver(() => {
      // 6-col grid with 8px gaps.
      const w = containerEl!.getBoundingClientRect().width;
      tileWidth = (w - 8 * 5) / 6;
    });
    ro.observe(containerEl);
    return () => ro.disconnect();
  });

  function onDragStart(_id: string) { /* no-op for now */ }

  async function onDragEnd(id: string, dx: number, _dy: number) {
    if (tileWidth <= 0) return;
    const slotsMoved = Math.round(dx / (tileWidth + 8));
    if (slotsMoved === 0) return;
    const idx = local.findIndex((p) => p.id === id);
    if (idx < 0) return;
    const target = Math.max(0, Math.min(local.length - 1, idx + slotsMoved));
    if (target === idx) return;

    const moved = [...local];
    const [it] = moved.splice(idx, 1);
    moved.splice(target, 0, it);
    local = moved.map((p, i) => ({ ...p, featured_position: i + 1 }));

    try {
      await reorderFeatured(fetch, local.map((p) => p.id));
    } catch (_e) {
      // Roll back on error.
      local = [...incoming];
    }
  }

  async function unpin(id: string) {
    const next = local.filter((p) => p.id !== id).map((p, i) => ({ ...p, featured_position: i + 1 }));
    local = next;
    try {
      await unpinFeatured(fetch, id);
    } catch (_e) {
      local = [...incoming];
    }
  }

  async function onPinned(photoId: string) {
    // Re-fetch is overkill — just synthesise an entry; the page will refresh on next nav.
    local = [
      ...local,
      {
        id: photoId,
        short_id: '',
        featured_position: (local.length + 1) as number,
        target: null,
        appreciations_count: 0,
        blurhash: null,
        width: null,
        height: null
      }
    ];
  }
</script>

{#if local.length > 0 || isOwner}
  <section class="row" bind:this={containerEl}>
    {#each local as item (item.id)}
      {#if editorMode}
        <FeaturedDraggableTile
          {item}
          {handle}
          {onDragStart}
          {onDragEnd}
          onUnpin={unpin}
        />
      {:else}
        <FeaturedTile {item} {handle} />
      {/if}
    {/each}
    {#each placeholders as i}
      <div class="slot">
        <span class="lab">SLOT {String(local.length + i + 1).padStart(2, '0')}</span>
        {#if i === 0 && editorMode}
          <button type="button" class="pin" onclick={() => (pinOpen = true)}>+ Pin a photo</button>
        {/if}
      </div>
    {/each}
  </section>
{/if}

{#if editorMode}
  <FeaturedPinModal
    bind:open={pinOpen}
    {handle}
    excludeIds={local.map((p) => p.id)}
    onPinned={(id) => void onPinned(id)}
  />
{/if}

<style>
  .row {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 8px;
    padding: 16px 32px;
    border-top: 1px solid var(--border-subtle);
  }
  .slot {
    aspect-ratio: 3 / 4;
    border: 1px dashed var(--border-subtle);
    display: flex; flex-direction: column; align-items: center; justify-content: center;
    gap: 8px;
    color: var(--fg-muted);
    font-family: var(--font-mono); font-size: 11px;
  }
  .pin {
    background: transparent;
    color: var(--accent);
    border: 1px solid var(--accent);
    padding: 6px 10px;
    font-family: var(--font-mono);
    font-size: 11px;
    cursor: pointer;
  }
  @media (max-width: 640px) { .row { grid-template-columns: repeat(2, 1fr); } }
</style>
```

(Synthesising the freshly-pinned tile with `short_id: ''` is intentional — the user is in editor mode and the local placeholder lives only until the next navigation. The plan's acceptance walk verifies that visiting the page after a pin shows the photo with the right short_id.)

- [ ] **Step 4: Skip commit.**

---

## Frontend — lightbox via shallow routing

**Routing decision:** SvelteKit shallow routing (`pushState` + `preloadData`) is the chosen mechanism. When a gallery tile or featured tile is clicked while the user is on `/u/<handle>`, intercept the navigation, preload the photo-detail data, and `pushState` a new URL of `/u/<handle>/p/<short_id>` with shallow state `{ lightbox: true, data: <preloaded> }`. The `+page.svelte` for that route inspects `$page.state` at render time: if `state.lightbox === true`, render inside `<Lightbox>` over the gallery; if not, render the full standalone photo-detail page that already exists. Direct visits (no shallow state) hit the full page.

This keeps deep links working both ways:
- Deep-link arrival → full photo-detail page (server `load` runs).
- Click-from-gallery → overlay lightbox; back button returns to gallery.

### Task 31: Lightbox.svelte + LightboxExifPanel.svelte + MoreFromPhotographerStrip.svelte

**Files:**
- Create: `frontend/src/lib/components/lightbox/Lightbox.svelte`
- Create: `frontend/src/lib/components/lightbox/LightboxExifPanel.svelte`
- Create: `frontend/src/lib/components/lightbox/MoreFromPhotographerStrip.svelte`

- [ ] **Step 1: LightboxExifPanel.svelte**

Pulls EXIF + equipment from the photo detail data. Sticky right column (380 px desktop), full-width bottom sheet on mobile.

```svelte
<script lang="ts">
  // The PhotoDetail shape exists in the existing photo-detail page;
  // import its type from wherever it lives. If the route uses an inline
  // type today, lift it into $lib/api/PhotoDetail.ts as part of this task.
  import type { PhotoDetail } from '$lib/api';

  let { photo }: { photo: PhotoDetail } = $props();
</script>

<aside class="panel">
  <h3 class="title">{photo.target ?? 'Untitled'}</h3>
  {#if photo.caption}<p class="caption">{photo.caption}</p>{/if}
  <slot name="appreciate" />

  <dl class="exif">
    {#if photo.camera}<div><dt>Camera</dt><dd>{photo.camera}</dd></div>{/if}
    {#if photo.lens}<div><dt>Lens</dt><dd>{photo.lens}</dd></div>{/if}
    {#if photo.iso}<div><dt>ISO</dt><dd>{photo.iso}</dd></div>{/if}
    {#if photo.exposure_seconds}<div><dt>Exposure</dt><dd>{photo.exposure_seconds}s</dd></div>{/if}
    {#if photo.taken_at}<div><dt>Taken</dt><dd>{new Date(photo.taken_at).toLocaleDateString()}</dd></div>{/if}
  </dl>

  <h4 class="subhead">Equipment</h4>
  <ul class="eq">
    {#if photo.scope}<li>Scope · {photo.scope}</li>{/if}
    {#if photo.mount}<li>Mount · {photo.mount}</li>{/if}
    {#if photo.filters}<li>Filters · {photo.filters}</li>{/if}
    {#if photo.guiding}<li>Guiding · {photo.guiding}</li>{/if}
  </ul>
  <slot name="more" />
</aside>

<style>
  .panel {
    width: 380px;
    max-width: 100%;
    padding: 24px;
    background: var(--bg-canvas);
    color: var(--fg-primary);
    overflow-y: auto;
  }
  .title { font-family: var(--font-display, serif); font-weight: 400; margin: 0 0 8px; }
  .caption { color: var(--fg-secondary); margin: 0 0 16px; }
  .exif { display: grid; grid-template-columns: 1fr 2fr; gap: 4px 12px; margin: 16px 0; font-family: var(--font-mono); font-size: 12px; }
  .exif dt { color: var(--fg-muted); }
  .subhead { font-family: var(--font-mono); font-size: 11px; color: var(--fg-muted); margin: 16px 0 4px; }
  .eq { list-style: none; padding: 0; margin: 0; font-family: var(--font-mono); font-size: 12px; color: var(--fg-secondary); }
  @media (max-width: 640px) {
    .panel { width: 100%; max-height: 50vh; }
  }
</style>
```

(`PhotoDetail` is whatever shape the existing `/u/[handle]/p/[short_id]/+page.server.ts` produces. If it's defined inline, lift it into `frontend/src/lib/api/PhotoDetail.ts` and re-export from `$lib/api`. Capture the actual property names from that file — the list above is illustrative.)

- [ ] **Step 2: MoreFromPhotographerStrip.svelte**

```svelte
<script lang="ts">
  import type { GalleryPhoto } from '$lib/api';
  import Img from '$lib/components/Img.svelte';

  let {
    handle,
    photos
  }: {
    handle: string;
    photos: GalleryPhoto[];
  } = $props();
</script>

{#if photos.length > 0}
  <section class="strip">
    <h4>More from <a href="/u/{handle}">@{handle}</a></h4>
    <ul>
      {#each photos as p (p.id)}
        <li>
          <a href="/u/{handle}/p/{p.short_id}" aria-label={p.target ?? 'Untitled'}>
            <Img photoId={p.id} w={200} aspectRatio="1/1" alt={p.target ?? 'Untitled'} class="img" />
          </a>
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .strip { padding: 16px 24px; border-top: 1px solid var(--border-subtle); }
  h4 { font-family: var(--font-mono); font-size: 11px; color: var(--fg-muted); margin: 0 0 8px; }
  h4 a { color: var(--accent); text-decoration: none; }
  ul { list-style: none; padding: 0; margin: 0; display: grid; grid-template-columns: repeat(4, 1fr); gap: 6px; }
  ul :global(.img) { width: 100%; aspect-ratio: 1/1; object-fit: cover; display: block; }
</style>
```

- [ ] **Step 3: Lightbox.svelte**

```svelte
<script lang="ts">
  import Img from '$lib/components/Img.svelte';
  import AppreciateButton from '$lib/components/AppreciateButton.svelte';
  import type { PhotoDetail } from '$lib/api';
  import LightboxExifPanel from './LightboxExifPanel.svelte';
  import MoreFromPhotographerStrip from './MoreFromPhotographerStrip.svelte';
  import type { GalleryPhoto } from '$lib/api';

  let {
    photo,
    handle,
    morePhotos = [],
    onClose,
    onPrev,
    onNext
  }: {
    photo: PhotoDetail;
    handle: string;
    morePhotos?: GalleryPhoto[];
    onClose: () => void;
    onPrev?: () => void;
    onNext?: () => void;
  } = $props();

  let panelExpanded = $state(true);

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
    else if (e.key === 'ArrowLeft' && onPrev) onPrev();
    else if (e.key === 'ArrowRight' && onNext) onNext();
    else if (e.key === 'i' || e.key === 'I') panelExpanded = !panelExpanded;
    else if (e.key === 'a' || e.key === 'A') {
      const btn = document.querySelector<HTMLButtonElement>('[data-appreciate-btn]');
      btn?.click();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="lightbox" role="dialog" aria-modal="true" aria-label="Photo viewer">
  <div class="image-pane">
    <Img photoId={photo.id} w={2400} alt={photo.target ?? 'Photo'} class="big" />
    <button type="button" class="close" aria-label="Close" onclick={onClose}>×</button>
    {#if onPrev}<button type="button" class="nav prev" aria-label="Previous" onclick={onPrev}>‹</button>{/if}
    {#if onNext}<button type="button" class="nav next" aria-label="Next" onclick={onNext}>›</button>{/if}
  </div>

  {#if panelExpanded}
    <LightboxExifPanel {photo}>
      <span slot="appreciate">
        <AppreciateButton photoId={photo.id} initial={photo.appreciations_count ?? 0} />
      </span>
      <svelte:fragment slot="more">
        {#if morePhotos.length > 0}
          <MoreFromPhotographerStrip {handle} photos={morePhotos} />
        {/if}
      </svelte:fragment>
    </LightboxExifPanel>
  {/if}
</div>

<style>
  .lightbox {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: grid;
    grid-template-columns: 1fr 380px;
    background: #000;
    color: #fff;
  }
  .image-pane {
    position: relative;
    background: #000;
    display: grid;
    place-items: center;
  }
  .image-pane :global(.big) {
    max-width: 100%;
    max-height: 100vh;
    object-fit: contain;
    display: block;
  }
  .close {
    position: absolute;
    top: 16px;
    right: 16px;
    background: rgba(255, 255, 255, 0.12);
    color: #fff;
    border: 0;
    width: 32px;
    height: 32px;
    cursor: pointer;
    font-size: 18px;
  }
  .nav {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    background: rgba(255, 255, 255, 0.1);
    color: #fff;
    border: 0;
    width: 40px;
    height: 64px;
    cursor: pointer;
    font-size: 24px;
  }
  .nav.prev { left: 12px; }
  .nav.next { right: 12px; }
  @media (max-width: 640px) {
    .lightbox { grid-template-columns: 1fr; grid-template-rows: 1fr auto; }
  }
</style>
```

(`AppreciateButton` already exists from Phase 7. The `data-appreciate-btn` query selector is a quick wire for the `a` shortcut — if that component already exposes a named ref, prefer that and update both ends.)

- [ ] **Step 4: Skip commit — bundled in Task 33.**

---

### Task 32: Wire shallow routing — gallery click opens lightbox

**Files:**
- Create: `frontend/src/lib/util/openLightbox.ts`
- Modify: `frontend/src/routes/u/[handle]/p/[short_id]/+page.svelte` (dual-render)
- Modify: `frontend/src/lib/components/profile/PhotoTile.svelte`, `FeaturedTile.svelte` (use the action)

- [ ] **Step 1: Create `frontend/src/lib/util/openLightbox.ts`**

```ts
import { preloadData, pushState } from '$app/navigation';
import type { Action } from 'svelte/action';

interface Options {
  short_id: string;
  handle: string;
}

/**
 * Click-intercept action for an <a> linking to /u/<handle>/p/<short_id>.
 * Modified clicks (cmd/ctrl/middle) navigate normally. Plain left-clicks
 * preload the route data and pushState into shallow lightbox state.
 */
export const openLightboxOnClick: Action<HTMLAnchorElement, Options> = (node, opts) => {
  let current = opts;

  function handler(e: MouseEvent) {
    if (e.button !== 0) return;
    if (e.metaKey || e.ctrlKey || e.altKey || e.shiftKey) return;
    e.preventDefault();
    void open();
  }

  async function open() {
    const url = `/u/${current.handle}/p/${current.short_id}`;
    const r = await preloadData(url);
    if (r.type !== 'loaded' || r.status !== 200) {
      window.location.href = url;
      return;
    }
    pushState(url, { lightbox: true, data: r.data });
  }

  node.addEventListener('click', handler);
  return {
    update(next) { current = next; },
    destroy() { node.removeEventListener('click', handler); }
  };
};
```

- [ ] **Step 2: Apply the action in `<PhotoTile>` and `<FeaturedTile>`**

In `frontend/src/lib/components/profile/PhotoTile.svelte`, change the opening anchor:

```svelte
<script lang="ts">
  import { openLightboxOnClick } from '$lib/util/openLightbox';
  // …existing imports
</script>

<a
  use:openLightboxOnClick={{ handle, short_id: photo.short_id }}
  href="/u/{handle}/p/{photo.short_id}"
  class="tile"
  …
>
```

Same change in `frontend/src/lib/components/profile/FeaturedTile.svelte`.

- [ ] **Step 3: Convert the photo-detail `+page.svelte` to dual-render**

Read the existing `frontend/src/routes/u/[handle]/p/[short_id]/+page.svelte`. If the markup is non-trivial, extract it into `frontend/src/lib/components/photos/PhotoDetailFull.svelte` with no behaviour change (verbatim move of the block, plus prop passthrough). Then write the page wrapper:

```svelte
<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import Lightbox from '$lib/components/lightbox/Lightbox.svelte';
  import PhotoDetailFull from '$lib/components/photos/PhotoDetailFull.svelte';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  let asLightbox = $derived(($page.state as { lightbox?: boolean } | null)?.lightbox === true);

  function close() {
    history.back();
  }
</script>

{#if asLightbox}
  <Lightbox
    photo={data.photo}
    handle={data.handle}
    morePhotos={data.morePhotos ?? []}
    onClose={close}
    onPrev={data.prevShortId ? () => goto(`/u/${data.handle}/p/${data.prevShortId}`) : undefined}
    onNext={data.nextShortId ? () => goto(`/u/${data.handle}/p/${data.nextShortId}`) : undefined}
  />
{:else}
  <PhotoDetailFull {data} />
{/if}
```

`data.morePhotos`, `data.prevShortId`, `data.nextShortId` come from the route's `+page.server.ts` — extend it in Task 34.

- [ ] **Step 4: Skip commit — bundle in Task 33.**

---

## Frontend — integration

### Task 33: Replace `/u/[handle]/+page.server.ts` and `+page.svelte`

**Files:**
- Modify: `frontend/src/routes/u/[handle]/+page.server.ts` (rewrite)
- Modify: `frontend/src/routes/u/[handle]/+page.svelte` (replace with `<HeroPage>` orchestration)

- [ ] **Step 1: Rewrite the server load to call the new aggregator**

```ts
import { error, redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { fetchPublicProfile, fetchPhotosFeed } from '$lib/api/profile';

export const load: PageServerLoad = async ({ params, fetch, locals }) => {
  const { handle } = params;

  let profile;
  try {
    profile = await fetchPublicProfile(fetch, handle);
  } catch (e) {
    if ((e as Error).message === 'not_found') {
      // Check redirect history.
      const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '';
      const r = await fetch(`${API}/api/handles/redirect/${handle}`);
      if (r.ok) {
        const { handle: target } = (await r.json()) as { handle: string };
        throw redirect(301, `/u/${target}`);
      }
      throw error(404, 'No photographer here.');
    }
    throw error(500, 'Profile lookup failed');
  }

  // First page of the gallery — SSR'd so the hero gallery has content on first paint.
  const firstPage = await fetchPhotosFeed(fetch, handle, { limit: 24 });

  const isSelf = locals.user?.id === profile.id;
  const viewMode: 'visitor' | 'owner' | 'admin' = isSelf ? 'owner' : 'visitor';

  return { profile, firstPage, viewMode };
};
```

- [ ] **Step 2: Rewrite the page**

```svelte
<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import HeroPage from '$lib/components/profile/HeroPage.svelte';
  import ProfileEditor from '$lib/components/profile/editor/ProfileEditor.svelte';
  import CoverPickerModal from '$lib/components/profile/editor/CoverPickerModal.svelte';
  import type { PageData } from './$types';
  import { invalidateAll } from '$app/navigation';

  let { data }: { data: PageData } = $props();

  let editorOpen = $state(false);
  let coverPickerOpen = $state(false);
  let pinTrigger = $state(0); // bumped when the pin modal is requested via FeaturedRow
</script>

<AppHeader />

<HeroPage
  profile={data.profile}
  viewMode={data.viewMode}
  onEditProfile={() => (editorOpen = true)}
  onPickCover={() => (coverPickerOpen = true)}
  onPinFirst={() => {
    // Open the editor at the featured section. Quick path: open the editor and
    // let the user scroll. A future polish ticket can deep-link to the section.
    editorOpen = true;
  }}
/>

{#if data.viewMode === 'owner'}
  <ProfileEditor
    bind:open={editorOpen}
    onSaved={() => void invalidateAll()}
  />
  <CoverPickerModal
    bind:open={coverPickerOpen}
    handle={data.profile.handle}
    onPicked={() => void invalidateAll()}
  />
{/if}

<AppFooter />
```

(Note: `<FeaturedRow>` from Task 30 owns its own pin modal in editor mode. The page itself doesn't need to pass a separate `editorMode` flag yet — Task 33's commit is the boundary at which the editor-mode wiring goes live; if `<HeroPage>` should expose `editorMode` so the FeaturedRow drag-controls activate, add a `editorMode={data.viewMode === 'owner'}` prop on the `<FeaturedRow>` invocation inside `<HeroPage>` and forward it down. This is one line.)

Quick fix in `<HeroPage>` (Task 15 file): change the `<FeaturedRow>` invocation to:

```svelte
<FeaturedRow
  items={profile.featured}
  handle={profile.handle}
  {isOwner}
  editorMode={isOwner}
  onPin={onPinFirst}
/>
```

`onPin` is no longer used (FeaturedRow's own modal handles pinning), so the prop can be deleted from `<HeroPage>`'s prop list and `<FeaturedRow>`'s prop list. The `onPinFirst` callback in `<HeroPage>` can be removed.

- [ ] **Step 3: Verify svelte-check is clean**

```
cd frontend && pnpm check
```

Expected: zero errors. If `bind:value={() => x, (v) => …}` getter/setter pair syntax is rejected by your Svelte version, replace those uses (in `LocationSection.svelte`, `ProfileEditor.svelte`) with explicit `value` + `onchange` callbacks.

- [ ] **Step 4: Sanity-test in the browser**

```
just dev
```

Open `http://localhost:5173/u/<your-handle>` and:
- Confirm the hero shell renders.
- As owner: open Edit profile, save tagline; reload, confirm persisted.
- As owner: open the cover picker; pick a photo; confirm cover renders.
- As owner: pin a photo via the placeholder; confirm it appears in the row.
- Drag a featured tile across two slots; confirm it persists on reload.
- Click a gallery tile; confirm the lightbox overlays without a hard nav.
- Press `Esc`; confirm back-button returns to the gallery URL.
- Direct-visit a `/u/<handle>/p/<short>` URL in a new tab; confirm the full photo-detail page renders (no lightbox).

Stop and surface any failure.

- [ ] **Step 5: Commit the entire frontend P2 in one go**

```
git add frontend/src/lib/ frontend/src/routes/
git commit -m "feat(frontend): photographer hero page + editor + lightbox (P2)

- 13-component hero shell on /u/<handle>: cover, identity, about,
  equipment strip, location badge, stats row, featured row, gallery
  toolbar, justified-rows photo grid.
- Profile editor modal (Tiptap configured to the bio allowlist),
  identity / about / equipment / location / social-links sections,
  save-on-blur per section.
- Cover picker modal + featured pin/unpin/drag-reorder controls.
- Lightbox via SvelteKit shallow routing: gallery clicks pushState
  into /u/<handle>/p/<short_id> with shallow state; deep visits
  render the existing standalone photo-detail page."
```

---

### Task 34: Extend `/u/[handle]/p/[short_id]/+page.server.ts` for prev/next + morePhotos

**Files:**
- Modify: `frontend/src/routes/u/[handle]/p/[short_id]/+page.server.ts`

- [ ] **Step 1: Read the existing load**

Note its current return shape and how it fetches the photo. Don't change behaviour beyond adding the new fields — the standalone page must keep working.

- [ ] **Step 2: Add `morePhotos`, `prevShortId`, `nextShortId`**

Inside the `load` after the existing fetches:

```ts
import { fetchPhotosFeed } from '$lib/api/profile';
// …

// "More from this photographer": 4 items, excluding the current one.
const feed = await fetchPhotosFeed(fetch, handle, { limit: 24 });
const others = feed.photos.filter((p) => p.id !== currentPhoto.id);
const morePhotos = others.slice(0, 4);

// Prev/next from the same feed for the lightbox arrows.
const idx = feed.photos.findIndex((p) => p.id === currentPhoto.id);
const prevShortId = idx > 0 ? feed.photos[idx - 1].short_id : null;
const nextShortId = idx >= 0 && idx < feed.photos.length - 1 ? feed.photos[idx + 1].short_id : null;

return {
  ...existingReturn,
  handle,
  morePhotos,
  prevShortId,
  nextShortId
};
```

(`existingReturn` is whatever the load currently produces — `{ photo: ..., owner: ..., ... }` — keep it intact.)

- [ ] **Step 3: Verify**

```
cd frontend && pnpm check
```

- [ ] **Step 4: Commit**

```
git add frontend/src/routes/u/'[handle]'/p/'[short_id]'/+page.server.ts
git commit -m "feat(frontend): photo-detail load returns morePhotos + prev/next short_ids"
```

---

## Quality gates and acceptance

### Task 35: Full quality-gate sweep

- [ ] **Step 1: Run `just check`**

```
just check
```

Expected: zero output. Fix anything it surfaces. Do NOT mark this step complete until output is clean.

- [ ] **Step 2: Run all backend tests**

```
cd backend && cargo test --tests
```

Expected: every suite green (P1 acceptance flagged `tests/photos_phase8b::replace_swaps_storage_key_keeps_metadata` as occasionally flaky in parallel — if it fails, run alone to confirm: `cargo test photos_phase8b::replace_swaps_storage_key_keeps_metadata -- --test-threads=1`).

- [ ] **Step 3: Run all frontend unit tests**

```
cd frontend && pnpm vitest run
```

Expected: all green, including the `tiptapAllowlist` drift test from Task 24 and `formatIntegration` from Task 14.

- [ ] **Step 4: Build the frontend**

```
cd frontend && pnpm build
```

Expected: clean build. SvelteKit's static asset / SSR build must not regress.

- [ ] **Step 5: Commit any prepare artefacts**

```
git status
```

If `.sqlx/` or `frontend/src/lib/api/*.ts` (generated) have uncommitted changes from this round of testing, commit them:

```
git add backend/.sqlx/ frontend/src/lib/api/
git commit -m "chore: refresh sqlx cache + ts-rs types after P2 work" || echo "nothing to commit"
```

---

### Task 36: Browser-driven acceptance walk via chrome-devtools-mcp

This task is a recorded interactive session — there is no Playwright spec. Drive a Chrome instance via the MCP tools available in this Claude Code worktree (`mcp__chrome-devtools__navigate_page`, `click`, `fill`, `take_screenshot`, etc.). Record findings in `docs/operations/p2-acceptance.md` using the table format from `p1-acceptance.md`.

**Steps to walk (each becomes a row in the acceptance table):**

1. New-tab `/signup`, complete signup with handle `marie2`.
2. Navigate to `/u/marie2`. Confirm visitor-as-owner empty-state hooks render: "Pick a cover from your gallery →", "Add a tagline", "Tell visitors about your astrophotography", "Add the gear behind your shots", "Where do you observe from?", featured "SLOT 01" with `[+ Pin a photo]`.
3. Click `Edit profile`. Fill display name, tagline, bio (use bold + italic + a link). Save by tabbing away.
4. Verify the fields persist by reloading the page.
5. Add equipment cells (RedCat 51 / ASI2600MC / ZWO AM5 / L-Pro / ASI120MM). Confirm the strip renders the populated cells and hides empty ones.
6. Set location, Bortle 6, SQM 19.8. Confirm the badge renders.
7. Add a Twitter and an Instagram link. Confirm icons render.
8. Open in a logged-out tab (different browser profile) and confirm visitor view: empty-state prompts hidden; the populated fields render.
9. Upload three photos via `/upload` (run through the existing P1 wizard). Publish them.
10. Reload the hero page. Confirm `frames=3` and the photos appear in the gallery in justified rows.
11. Open the cover picker, pick photo #1, confirm it renders as the cover with the `● COVER · <target>` credit.
12. Pin photos 1, 2, 3 via the featured slot 01 affordance. Confirm slots 04..06 still render placeholders.
13. Drag photo 1 from slot 1 to slot 3. Reload; confirm the new order (3, 2, 1).
14. Unpin photo 2 via the hover ✕. Confirm slots compact (now 1, 3, with placeholders 03..06).
15. Click a gallery tile. Confirm the lightbox overlays the page (URL changes; gallery still visible behind).
16. Press `→` then `←` in the lightbox; confirm prev/next swap the image.
17. Press `i`; confirm the EXIF panel collapses, then reopens.
18. Press `a`; confirm the appreciate count increments.
19. Press `Esc`; confirm the lightbox closes and the URL returns to `/u/marie2`.
20. Reload at `/u/marie2/p/<short>` directly; confirm the full photo-detail page renders (no lightbox overlay).
21. Inspect a sample bio with a `<script>` tag pasted via DevTools `evaluate_script` PATCH; confirm the saved value comes back without `<script>` (sanitiser).
22. Verify a malformed `social_links` POST (`{"platform":"twitter","url":"https://evil.example/marie"}`) returns 400.
23. Verify an attempt to pin a 7th photo returns 409.
24. Verify a cross-owner pin attempt (use a second account's published photo id from the `b` user) returns 404.

- [ ] **Step 1: Run the walk and record results**

Open Chrome via MCP, work through the 24 steps. For each step record `pass`/`fail` (with the failure mode) in a new file `docs/operations/p2-acceptance.md` (use `p1-acceptance.md` as the template — same headers, same column shape).

- [ ] **Step 2: Capture three diagnostic screenshots into `docs/operations/`**

Names suggested by the spec:
- `p2-hero-owner.png` — owner-mode banner + populated hero.
- `p2-hero-visitor.png` — visitor view of the same profile.
- `p2-lightbox.png` — lightbox open over a justified-rows gallery.

Save into `docs/operations/screenshots/p2/`. Reference from the acceptance doc.

- [ ] **Step 3: Commit the acceptance doc + screenshots**

```
git add docs/operations/p2-acceptance.md docs/operations/screenshots/
git commit -m "docs(ops): P2 acceptance report (chrome-devtools-mcp E2E)"
```

If any of the 24 walk steps failed: do NOT continue to Task 37. Open a focused fix commit, re-walk only the affected step, then re-record.

---

### Task 37: Push, open the PR, merge, clean up

The branch `feat/showcase-p2-hero` is ready. Mirror the P1 cadence.

- [ ] **Step 1: Push the branch**

```
git push -u origin feat/showcase-p2-hero
```

- [ ] **Step 2: Open the PR**

```
gh pr create --base main --head feat/showcase-p2-hero --title "feat: photographer showcase P2 (hero page)" --body "$(cat <<'EOF'
## Summary

Phase 2 of the **Photographer Showcase** — `/u/<handle>` rebuilt as a polished public profile, plus the profile editor, cover picker, featured-photo controls, justified-rows gallery, and shallow-routed lightbox. Schema and core endpoints landed in P1; this PR adds the remaining write endpoints (cover, featured pin/unpin/reorder, public-profile aggregator, gallery feed) and the entire UI surface.

- **Backend:** four new endpoints — `POST /api/me/cover`, `POST/DELETE /api/me/featured/:photo_id`, `PATCH /api/me/featured/order`, plus `GET /api/users/by-handle/:handle/profile` (aggregator) and `/photos` (cursor-paginated feed). Extended `PATCH /api/me/profile` to write every P2 field with bio sanitisation (ammonia, shared JSON allowlist with the Tiptap editor) and typed `social_links` validation. Featured pin/unpin/reorder uses staged NULL-then-target writes to preserve the partial-unique constraint on `(owner_id, featured_position)`.
- **Frontend:** 13-component hero shell (cover, identity, about, equipment strip, location badge, stats row, featured row, gallery toolbar, photo grid), Tiptap-backed profile editor, cover picker, drag-reorderable featured row (`@neodrag/svelte`), justified-rows gallery (Flickr's `justified-layout`), lightbox via SvelteKit shallow routing (`pushState` + `preloadData`).
- **Tests:** 30+ new backend integration tests (profile_extended, cover_set, featured_pin, featured_reorder, public_profile, photos_feed) plus drift unit tests for the Tiptap allowlist and the integration formatter.

## References

- Spec: \`docs/superpowers/specs/2026-05-03-photographer-showcase-design.md\` (P2 section)
- Plan: \`docs/superpowers/plans/2026-05-03-photographer-showcase-p2-hero-page.md\`
- Acceptance: \`docs/operations/p2-acceptance.md\` (chrome-devtools-mcp walk)

## Test plan

- [x] \`cd backend && cargo test --tests\` — all suites green
- [x] \`just check\` — fmt + clippy + svelte-check clean
- [x] \`pnpm vitest run\` — Tiptap-allowlist drift test + format helpers green
- [x] Browser walk recorded in p2-acceptance.md (24 steps, all pass)

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

Capture the PR URL.

- [ ] **Step 3: Wait for the gitleaks check**

```
gh pr view <PR#> --repo 100-tokens/astrophoto --json statusCheckRollup
```

Expected: gitleaks `SUCCESS`. If it fails, surface the leak and fix before merging.

- [ ] **Step 4: Merge with a merge commit (matches P1 cadence)**

```
gh pr merge <PR#> --repo 100-tokens/astrophoto --merge
```

Expected: `state: MERGED`. Capture the merge SHA.

- [ ] **Step 5: After-merge cleanup (manual — `delete_branch_on_merge: false`)**

```
# Delete remote branch.
git push origin --delete feat/showcase-p2-hero

# From inside the worktree, fast-forward main, then leave the worktree.
git pull --ff-only origin main
cd ..

# Remove the worktree directory and the local branch.
git -C /Volumes/Pascal4Tb/Projects/astrophoto worktree remove /Volumes/Pascal4Tb/Projects/astrophoto-showcase-p2
git -C /Volumes/Pascal4Tb/Projects/astrophoto branch -d feat/showcase-p2-hero
```

If `git branch -d` complains "not fully merged", that means local `main` hasn't seen the merge commit yet — `git pull --ff-only origin main` from the *main* worktree (not this one), then retry the `-d`.

- [ ] **Step 6: Final commit confirmation**

```
git -C /Volumes/Pascal4Tb/Projects/astrophoto log --oneline -5
```

Expected: a `Merge pull request #N from 100-tokens/feat/showcase-p2-hero` commit at the tip.

---

## After merge — what's next

P3 (Discovery) is the next phase in the photographer showcase plan. It rides on the schema + plumbing this PR shipped (taxonomy from P1; appreciations counter; gallery feed shape) — no new migrations expected. Routes to be designed: `/explore`, `/t/<slug>`, `/equip/<...>`, search, the cross-author photo grid surface. Brainstorm and plan via the same skill chain (`brainstorming` → `writing-plans`) when ready.

---

## Pre-existing items observed during P2 work, not addressed here

- The `bind:value={() => …, (v) => …}` getter/setter pair is the Svelte 5 way to forward bidirectional binding into a child without exposing the parent's internal `$state`. If your Svelte version's syntax differs (older betas accepted only the simple form), see Step 3 of Task 33 for the fallback pattern.
- `@tiptap/extension-underline` is a separate package because starter-kit doesn't include the `Underline` mark.
- `Modal.svelte`'s `a11y_no_static_element_interactions` warning from P1 is still pre-existing; not in scope for P2.
