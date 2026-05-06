# Celestial Objects Design — OpenNGC Catalog + Multi-Target Tagging

**Date:** 2026-05-06
**Status:** Draft — pending written-spec review
**Author:** Pascal (with Claude)

## Goal

Turn the existing `targets` table from a Messier-only stub (≈120 rows,
no astronomical metadata) into a real celestial-object catalog backed
by **OpenNGC** (≈14 000 galaxies, nebulae, clusters with RA/Dec, type,
constellation, magnitude, dimensions). Surface that data on
`/t/<slug>` pages and make it useful at upload time by replacing the
single-target picker with a **multi-target** picker that uses the
already-existing many-to-many join.

The feature ships as **one spec, one implementation plan** covering
four tracks (D1, D2a, D2b, D2c). Aladin Lite embed, NASA/ESA gallery,
and plate solving are explicitly deferred to D3+ — see "Out of scope".

## Why now

The `targets` / `photo_targets` schema and the `/t/<slug>` page were
shipped during the photographer-showcase discovery phase
(commits `b48df65` and `b636be0`). The infra is in place but the
**content is starved**: only Messier 1→110 plus eight popular NGC/IC
objects are seeded, with zero astronomical metadata. The page header
shows just `"Messier 5 · 12 photos"` — no RA/Dec, no constellation, no
type. And the upload picker is single-select, even though the join
table already supports multi.

Targets are the **subject** facet of a photo (vs photographer = who,
equipment = how). After photographer-showcase (who) and equipment
setups (how, in flight), enriching the subject facet is the natural
next step and the strongest differentiator of an astrophotography
platform vs a generic photo site.

## Decisions

| #   | Topic                              | Choice                                                                                          |
| --- | ---------------------------------- | ----------------------------------------------------------------------------------------------- |
| 1   | Catalog source                     | **OpenNGC** (`mattiaverga/OpenNGC`, CC-BY-SA 4.0). NGC.csv + addendum.csv pinned in `backend/data/openngc/`. |
| 2   | URL scheme                         | **Keep `/t/<slug>`** — already shipped, indexed, linked. No rename.                            |
| 3   | Tagging cardinality                | **Multi-target, optional, with free-text fallback** (max 5 per photo).                         |
| 4   | Page content (D2)                  | **Metadata header + community grid**. No descriptions, no Wikipedia, no AI text in this slice. |
| 5   | Import mechanism                   | **Rust binary `seed-targets` + `just seed-targets`** recipe. Not in migration. Re-runnable.    |
| 6   | Schema strategy                    | All new astro columns **nullable**. Preserves manual rows (M40, M45) and seeds without astro metadata. |
| 7   | Merge with existing seed           | UPSERT by slug, **never overwrite** `canonical_name` / `aliases` (manual overrides preserved). |
| 8   | Special cases                      | `KEEP_MANUAL_META = {'ic-434'}` skip-list in seed binary (verified against OpenNGC 36cb178, 2026-04-16 — see Risks). M40 and M45 handled correctly via addendum.csv. |
| 9   | Object-type / constellation labels | Lookup tables in frontend (`$lib/data/celestial.ts`), not stored in DB. Codes (G, GCl, AND…) stay in DB. |
| 10  | Multi-target write API             | At upload: extend existing `POST /api/photos/:id/metadata` with optional `targets` array (single atomic call, no silent-failure window). Separate `PATCH /api/photos/:id/targets` for post-publish edits. Both delete existing `source='manual'` rows and re-insert; preserve `source='plate_solve'`. |
| 11  | Backfill of existing photos        | **`just backfill-photo-targets`** one-shot binary, dry-run by default. Run manually post-deploy on staging then prod. |
| 12  | Index page `/t`                    | New SSR route. Filters by object_type + constellation, search across slug/canonical_name/aliases, sort by popularity / name / recent. |
| 13  | Search implementation              | `ILIKE` over canonical_name, slug, aliases. No `pg_trgm` for now (over-engineering at 14k rows). |
| 14  | i18n                               | French-only labels initially. When project-wide i18n lands, lookup tables migrate. |

---

## Architecture overview

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│ OpenNGC CSV     │────▶│ seed-targets bin │────▶│  targets table  │
│ (pinned in repo)│     │ (UPSERT + merge) │     │  ~14k rows      │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                                                         ▲
                                                         │
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Upload UI      │────▶│ PATCH /photos/   │────▶│ photo_targets   │
│ TargetMulti-    │     │ :id/targets      │     │ (M2M join,      │
│ Picker          │     │                  │     │  source=manual) │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                                                         ▲
                                                         │
┌─────────────────┐     ┌──────────────────┐             │
│  /t/<slug>      │◀────│ GET /api/        │─────────────┘
│  page (header   │     │ targets/:slug    │
│  + photo grid)  │     │ (enriched)       │
└─────────────────┘     └──────────────────┘
                                                         ▲
┌─────────────────┐     ┌──────────────────┐             │
│  /t (index)     │◀────│ GET /api/targets │─────────────┘
│  (browse all)   │     │ (paginated list) │
└─────────────────┘     └──────────────────┘
```

**Components touched:**
- New: migration `0014_targets_astro_meta.sql`, binary
  `backend/src/bin/seed_targets.rs`, binary
  `backend/src/bin/backfill_photo_targets.rs`,
  `frontend/src/lib/components/TargetMultiPicker.svelte`,
  `frontend/src/lib/data/celestial.ts`,
  `frontend/src/lib/utils/coords.ts`,
  `frontend/src/routes/t/+page.{svelte,server.ts}`.
- New endpoints: `PATCH /api/photos/:id/targets`, `GET /api/targets`.
- Modified: `discovery/target.rs` (extend `TargetMeta`), api_types,
  `DiscoveryHeader.svelte` (variant=`target` branch),
  `upload/[id]/verify/+page.svelte` (swap picker), justfile.

---

## D1.1 — Schema migration

**File:** `backend/migrations/0014_targets_astro_meta.sql`

```sql
alter table targets
  add column right_ascension   double precision,
  add column declination       double precision,
  add column magnitude_v       real,
  add column object_type       text,
  add column constellation     char(3),
  add column major_axis_arcmin real,
  add column minor_axis_arcmin real,
  add column updated_at        timestamptz not null default now();

create index targets_object_type_idx
    on targets (object_type)  where object_type is not null;
create index targets_constellation_idx
    on targets (constellation) where constellation is not null;
```

**Notes:**
- All astro columns nullable — OpenNGC does not cover every existing
  row (M40 = Winnecke 4 binary, in `addendum.csv`; M45 Pleiades cluster
  is not in NGC core — both handled via addendum). `ic-434` is in
  `KEEP_MANUAL_META` and stays nullable on astro fields (see Risks).
- `kind` (catalog provenance: messier/ngc/ic/…) stays distinct from
  `object_type` (astronomical type: G/PN/OCl/…). They are not
  synonymous.
- No SQL enum for `object_type` — keeps schema flexible vs OpenNGC
  evolution.
- `updated_at` to track when seed last touched a row.
- No spatial index (cube/earthdistance) yet — deferred to D6 cone search.

---

## D1.2 — OpenNGC import

**Source files committed under `backend/data/openngc/`:**
- `NGC.csv` — main catalog (~14k rows, ~3 MB)
- `addendum.csv` — Messier objects not in NGC/IC (M40, M45, M24…)
- `README.md` — version pin, source URL, license attribution

**Binary:** `backend/src/bin/seed_targets.rs`. Invoked via
`just seed-targets` which runs `cargo run --bin seed-targets --release`.

**Algorithm (per CSV row):**

```
1. Parse fields: Name, M, RA, Dec, Type, Const, V-Mag, MajAx, MinAx, CommonNames
2. Compute slug:
   - if M is set     → "m{M}"          (e.g. m31)
   - else if Name matches `^NGC(\d+)$` → "ngc-{num}" (zero-padding stripped)
   - else if Name matches `^IC(\d+)$`  → "ic-{num}"
   - else            → skip. This excludes:
       * Subcomponent rows like `NGC5128A`, `NGC0292A` (galaxy
         components / double objects). The parent NGC entry is
         present separately.
       * Stars, novae, non-NGC/IC catalog entries.
   - Skipped rows are counted and logged (not silent).
3. UPSERT by slug:
   - Slugs strip OpenNGC zero-padding: "NGC0224" → "ngc-224", not
     "ngc-0224" (matches existing seed style).
   - INSERT ... ON CONFLICT (slug) DO UPDATE
   - UPDATE only:  ra, dec, magnitude_v, object_type,
                   constellation, major_axis_arcmin,
                   minor_axis_arcmin, updated_at
   - NEVER overwrite canonical_name (preserves manual overrides from
                   migration 0010 like "Andromeda Galaxy" on m31).
   - aliases are EXTENDED, never replaced: existing entries preserved,
                   new catalog forms ("NGC 224" / "M 31" / "IC 434")
                   appended via `array_cat` + dedup if absent.
   - On INSERT only: canonical_name = first CommonName or `Name`,
                     kind derived from prefix.
4. KEEP_MANUAL_META skip-list: ['ic-434']
   - For these slugs, do not UPDATE astro metadata fields (ra, dec,
     object_type, constellation, dimensions).
   - `ic-434`: OpenNGC IC0434 = HII emission nebula ("Flame Nebula,
     Orion B"), but our slug refers to the Horsehead Nebula (Barnard 33,
     a dark nebula silhouetted against IC0434). Updating object_type
     to 'HII' on a row named "Horsehead Nebula" is factually wrong.
   - `m45` is NOT on this list: the binary maps m45 via addendum Mel022
     (M=045, OCl, "Pleiades"), which is correct. NGC1432 "Maia Nebula"
     has no M field → slugs as "ngc-1432", never touches m45.
   - Also skip rows where Type='Dup' (see addendum M102 note in Risks).
5. Second pass: addendum.csv → fill rows that core CSV did not match.
```

**Idempotence:** running the binary N times converges to the same
state. Manual edits to `canonical_name` / `aliases` survive every
re-run.

**Performance:** ~14k UPSERTs in a single transaction, batched at 500
rows. Expected <2s on local Postgres. Not a critical path.

**Tests:**
- Unit: parser fixtures (5 representative rows: galaxy with M number,
  IC alone, star to skip, missing V-Mag, missing dimensions).
- Unit: merge logic — given seed-0010 state, after running the binary
  on a 3-row fixture, M31 has `right_ascension ≈ 10.68`,
  `object_type='G'`, `canonical_name` still `Andromeda Galaxy`.
- Integration: testcontainer Postgres → run migrations → run binary →
  assert `count(*) ≥ 13800`,
  `count(*) where ra is null and slug like 'ngc-%' = 0`,
  `m45.canonical_name = 'Pleiades'` and `m45.object_type = 'OCl'`,
  `ic-434.canonical_name = 'Horsehead Nebula'` and `ic-434.object_type IS NULL`.

**License attribution (CC-BY-SA 4.0 four required elements):**
small footer on `/t/<slug>` and `/t` pages with all four:
- Attribution: "OpenNGC by Mattia Verga and contributors"
- License link: https://creativecommons.org/licenses/by-sa/4.0/
- Source link: https://github.com/mattiaverga/OpenNGC
- Change indication: "Adapted to slug format and merged with manual
  catalog seed."

---

## D2a — Multi-target picker at upload

### Frontend component

New `frontend/src/lib/components/TargetMultiPicker.svelte`:

```svelte
<TargetMultiPicker
  bind:targets={selectedTargets}    // Array<{ slug, canonical_name, kind }>
  bind:primary={primarySlug}        // string | null
  max={5}
/>
```

Visual: stacked chips with star marker on the primary, autocomplete
input below to add, free-text fallback input at the bottom (used only
when the chips list is empty). Clicking a non-primary chip promotes it
to primary.

Implementation: extracts `<TargetAutocompleteInput>` from existing
`<TargetPicker>` as a shared sub-component. `<TargetPicker>` (mono)
keeps its current API; `<TargetMultiPicker>` wraps the autocomplete
and manages the chip list.

### Backend — atomic write via existing metadata endpoint

**Critical**: the spec does **not** introduce a two-call upload flow.
A separate PATCH after metadata POST creates a silent-failure window
where the primary chip survives via the legacy free-text path but
secondary chips are dropped if the second call fails.

Instead: extend the existing `POST /api/photos/:id/metadata` request
to accept an **optional** `targets` array. When present, it is the
source of truth and the legacy `attach_primary_by_freetext` is
**suppressed** for that request.

**Request body addition:**
```json
{
  "...existing fields...": "...",
  "target": "Orion Nebula",
  "targets": ["m42", "ngc-1977", "ic-434"]
}
```

**Behavior of metadata handler when `targets` is present (transactional
within the existing metadata transaction):**
1. Validate every slug exists → 400 with offending slug.
2. Reject duplicate slugs in the array → 400.
3. Reject `targets.len() > 5` → 400.
4. `DELETE FROM photo_targets WHERE photo_id = $1 AND source = 'manual'`
   (preserves `source='plate_solve'` rows for D5+).
5. Insert each slug with `source='manual'`; the **first** entry gets
   `is_primary=true`, the rest `false`.
6. **Skip** the call to `attach_primary_by_freetext` for this request.

When `targets` is omitted or `null` → existing behavior unchanged
(`attach_primary_by_freetext` runs against the `target` text field).

### Separate endpoint for post-publish edits

`PATCH /api/photos/:id/targets` exists for the case "user already
published, wants to edit the target list later" (from a future photo
edit UI, not part of this slice).

Same body and behavior as the in-metadata path:
```json
{ "targets": ["m42", "ngc-1977"] }
```

**Response (both paths):** `200 { targets: [{ slug, canonical_name,
is_primary }] }`. **Manual-source rows only** — plate_solve rows are
not returned by this endpoint, even though they remain in DB.

### Upload flow integration

`upload/[id]/verify/+page.svelte` currently uses `<TargetPicker>` bound
to a single `target` text. Change:
- Replace `<TargetPicker bind:value={target} />` with
  `<TargetMultiPicker bind:targets bind:primary />`.
- On submit, the client computes the legacy `target` text field as
  follows: if at least one chip is selected → `target` =
  primary's `canonical_name`; else → `target` = the free-text fallback
  input.
- The metadata POST body includes `targets: <slug array>` when chips
  are present, `null` otherwise. **Single network call**, transactional
  on the server side. No silent-failure window.

### Multi-picker UX edge cases

- **Removing the primary chip**: the next chip in list order auto-
  promotes to primary. If the list becomes empty, `primary = null`
  and the free-text fallback input becomes active again.
- **Adding a chip when at max=5**: the autocomplete input becomes
  read-only with a hint "5 sujets max". User must remove one before
  adding another.
- **Free-text fallback while chips are present**: input is shown but
  disabled, with hint "Utilisé seulement si aucun objet sélectionné".

### Tests

- Unit Rust: idempotent re-PATCH yields same join rows; plate_solve
  rows preserved; primary correctly positioned.
- Integration: 403 on wrong owner, 400 on unknown slug, 400 on
  duplicate, 400 on >5.
- E2E (chrome-devtools): upload → tag M42 + NGC 1977 → verify both
  `/t/m42` and `/t/ngc-1977` list the photo.

---

## D2b — Enriched page header

### API extension

`api_types::TargetMeta` gains:
```rust
pub right_ascension:   Option<f64>,
pub declination:       Option<f64>,
pub magnitude_v:       Option<f32>,
pub object_type:       Option<String>,
pub constellation:     Option<String>,
pub major_axis_arcmin: Option<f32>,
pub minor_axis_arcmin: Option<f32>,
```

`discovery/target.rs::get` query updated to select the new columns.

### Frontend

`<DiscoveryHeader variant="target">` branch updated to render:

```
M31 · Andromeda Galaxy
Galaxie  ·  Andromède  ·  RA 00ʰ42ᵐ44ˢ  ·  Dec +41°16′09″
mag 3.4  ·  190′ × 60′  ·  alias : NGC 224, Messier 31

47 photos  ·  23 photographes
```

All fields optional. If everything astro is null, only the slug,
canonical_name, and counts render — i.e. behavior reverts to the
current header.

**New helpers:**
- `$lib/utils/coords.ts`:
  - `formatRA(degrees: number): string` → `"00ʰ42ᵐ44ˢ"`
  - `formatDec(degrees: number): string` → `"+41°16′09″"` (signed, sexagesimal)
  - Pure functions, unit-tested with 5–6 cases (M31, M42, NGC 7000,
    deep-south negative, value near 0).
- `$lib/data/celestial.ts`:
  - `OBJECT_TYPE_LABELS: Record<string, string>` — all OpenNGC type
    codes (G, GCl, OCl, PN, SNR, Neb, HII, Cl, *Ass, **, …)
  - `CONSTELLATION_LABELS: Record<string, string>` — 88 IAU codes
    (AND→Andromède, ORI→Orion, …).
  - Unknown code → display the raw code (safe fallback).

### Tests

- Unit TS: `formatRA`/`formatDec` cases.
- Integration Rust: `/api/targets/m31` after seed returns
  `object_type='G'`, `constellation='AND'`, `magnitude_v ≈ 3.4`.
- Visual check: `/t/m31` displays the new lines; `/t/ic-434`
  (KEEP_MANUAL_META) shows minimal header (only slug + canonical_name +
  counts, no astro metadata); `/t/m45` shows enriched header with
  `OCl` type and Tau constellation (m45 is NOT on the skip-list).

---

## D2c — Index page `/t`

### Route

New SSR route at `frontend/src/routes/t/+page.{svelte,server.ts}`,
above the existing `/t/[slug]`.

URL: `/t?object_type=G&constellation=ORI&sort=popular&q=&cursor=…`
(query param names match the API fields below; values are OpenNGC
codes, not localized labels.)

### Backend endpoint

New: `GET /api/targets` in `discovery/target_index.rs` (new file).

**Query params:**
```rust
struct ListQ {
    q:             Option<String>,   // search across canonical_name, slug, aliases
    object_type:   Option<String>,   // 'G', 'PN', …
    constellation: Option<String>,   // 'ORI', 'AND', …
    sort:          Option<String>,   // 'popular' (default) | 'name'
    cursor:        Option<String>,   // base64 (sort_value, id)
    limit:         Option<i64>,      // default 24, max 60
}
```

**Response:**
```json
{
  "targets": [
    {
      "slug": "m42",
      "canonical_name": "Orion Nebula",
      "object_type": "Neb",
      "constellation": "ORI",
      "magnitude_v": 4.0,
      "photo_count": 87,
      "preview_thumbs": [
        { "short_id": "abc12", "blurhash": "L6P…" },
        { "short_id": "def34", "blurhash": "L8Q…" },
        { "short_id": "ghi56", "blurhash": "L7R…" }
      ]
    }
  ],
  "next_cursor": "…"
}
```

`preview_thumbs`: top-3 most-appreciated photos for that target via
`LATERAL` subquery against `photo_targets` join. Up to 3 entries; can
be empty for targets with zero photos.

**Search query (no `pg_trgm` for now):**
```sql
where ($1::text is null
       or canonical_name ilike '%' || $1 || '%'
       or slug ilike '%' || $1 || '%'
       or exists (select 1 from unnest(aliases) a where a ilike '%' || $1 || '%'))
```

**Cursor encoding:**
- `popular` → `(photo_count desc, id desc)`
- `name`    → `(canonical_name asc, id asc)`

(No "recent" sort — `targets.updated_at` is touched on every seed
re-run, which would make all rows tie ≈ deploy time. A "most-recently
photographed" sort is interesting but requires a `MAX(p.published_at)`
join; deferred to a later iteration.)

### Page layout

```
┌─────────────────────────────────────────────────────────────┐
│ Objets célestes                            [recherche…    ] │
│ Type: [Tous ▾] [Galaxie] [Nébuleuse] [Amas ouvert] [PN]     │
│ Constellation: [Toutes ▾] [Orion] [Andromède] …             │
│ Tri: [Populaire ▾] [Alphabétique]                            │
└─────────────────────────────────────────────────────────────┘
┌──────────────┐ ┌──────────────┐ ┌──────────────┐
│ ▣ ▣ ▣        │ │ ▣ ▣ ▣        │ │ ▣ ▣ ▣        │
│ M31          │ │ M42          │ │ NGC 7000     │
│ Andromeda    │ │ Orion Nebula │ │ N. America   │
│ Galaxie · AND│ │ Néb · ORI    │ │ Néb · CYG    │
│ 47 photos    │ │ 87 photos    │ │ 31 photos    │
└──────────────┘ └──────────────┘ └──────────────┘
                  [Charger plus]
```

Cards link to `/t/<slug>`. Mini-strip of 3 thumb placeholders renders
from blurhash, lazy-loaded images on viewport entry. Filter pills
reuse `<FilterPills>` adapted with `variant="target-index"`.

Search input debounced 200ms; updates `?q=` query param;
SvelteKit `goto({ replaceState, keepFocus, noScroll })` re-runs
`load`.

### Empty states

- No filters, full catalog → 24 most-popular targets.
- Filters + zero results → "Aucun objet ne correspond. [Effacer les filtres]"
- Target with zero photos → still shown when no popularity sort,
  thumbs strip becomes a gray placeholder, label "0 photos · Soyez le
  premier".

### SEO

- `<title>`: "Objets célestes — Astrophoto"
- `<meta name="description">`: "Explore 14 000 galaxies, nébuleuses
  et amas photographiés par la communauté."
- SSR-rendered (server `load`) for crawlability.

### Tests

- Unit Rust: cursor encode/decode for both sort modes.
- Integration: filter by `object_type=G` + `constellation=AND`
  returns M31; search `q=andromed` returns M31.
- E2E: open `/t`, type "orion", click M42 → header shows enriched
  metadata.

---

## Backfill of existing photos

Before D1, `attach_primary_by_freetext` could only match against ~120
catalog rows. After D1, ~14k rows are matchable — many `photos.target`
text values that previously fell through can now resolve.

**One-shot binary `backend/src/bin/backfill_photo_targets.rs`,
invoked via `just backfill-photo-targets`:**

- Selects `photos where target is not null and target <> '' and not
  exists (select 1 from photo_targets pt where pt.photo_id = photos.id
  and pt.source = 'manual')`.
- For each row, runs the same lookup as `attach_primary_by_freetext`
  (slug exact / alias / canonical_name ilike).
- Default mode: dry-run. Logs match / no-match / ambiguous counts.
- `--apply` flag actually writes the join rows.
- Idempotent: re-running with `--apply` is a no-op (the existence
  guard skips photos that already have a manual row).

**Run cadence:** manually post-deploy on staging then prod, output
appended to `docs/operations/p?-acceptance.md`. Not part of CI.

**Tests:**
- Integration on testcontainer: 5-row fixture with mix of resolvable
  and ambiguous text targets. Dry-run produces expected
  match/no-match counts. `--apply` writes the join rows; second
  `--apply` is a no-op (idempotent).

---

## Rollout plan

1. Land migration `0014` (additive, nullable cols → safe).
2. Deploy backend exposing new fields and `PATCH` endpoint.
3. Run `just seed-targets` on staging, smoke-check
   `count(*) ≈ 13800` and `/api/targets/m31` returns enriched data.
4. Deploy frontend (`<TargetMultiPicker>`, header, `/t` index).
5. `just backfill-photo-targets --apply` on staging → review →
   replay on prod.

Each step is independent and observable. Rollback at any step =
revert the deploy; nullable columns and additive endpoints leave no
data debt behind.

---

## Risks

- **CSV pinning drift:** OpenNGC publishes ~1–2 updates per year.
  Stale data is not breaking, just out-of-date. Mitigation: annual
  refresh PR, run `just seed-targets` again.
- **Manual-meta protection (KEEP_MANUAL_META):** verified against
  OpenNGC commit 36cb178 (2026-04-16) on 2026-05-06. All seed-0010
  slugs were checked against NGC.csv and addendum.csv.

  **Only `ic-434` requires astro-metadata protection:**
  - `ic-434`: OpenNGC IC0434 = HII emission nebula, common names
    "Flame Nebula, Orion B". Our canonical_name is "Horsehead Nebula"
    (Barnard 33, a dark nebula silhouetted *in front of* IC0434).
    Updating `object_type` to 'HII' on a row named "Horsehead Nebula"
    is factually wrong — the Horsehead is a dark nebula (DrkN), not an
    HII region. The true Horsehead (B033) is only in addendum.csv with
    no M or NGC/IC identifier, so the binary cannot slug it and the
    ic-434 row stays as the canonical "Horsehead region" entry.

  **m45 is NOT on the skip-list** (original spec was wrong): NGC1432
  "Maia Nebula" has an empty M field in OpenNGC → the binary slugs it
  as `ngc-1432`, not `m45`. The `m45` slug comes exclusively from
  addendum row Mel022 (M=045, type=OCl, common name "Pleiades"), which
  is entirely correct.

  **All other manual-override slugs are safe to receive astro updates:**
  - `m31` (NGC0224, G, "Andromeda Galaxy"), `m42` (NGC1976, Cl+N,
    "Orion Nebula"), `m33` (NGC0598, G, "Triangulum Galaxy"), `m51`
    (NGC5194, G, "Whirlpool Galaxy"), `m27` (NGC6853, PN, "Dumbbell
    Nebula"), `m13` (NGC6205, GCl, "Hercules Globular Cluster"):
    OpenNGC common names match or are consistent with our canonical names.
  - `ngc-7000` ("North America Nebula"), `ngc-6960` ("Western Veil" in
    OpenNGC common names), `ngc-2237` ("Rosette A" = same physical
    complex): safe.
  - `ngc-281`, `ngc-3324`, `ic-1805`, `ic-1396`: OpenNGC has no common
    name for these rows; our canonical names are preserved automatically.
  - `m40` (addendum: double star `**`, no common name), `m45` (addendum:
    Mel022, OCl, "Pleiades"), `m73` (NGC6994, type "Other", 4 galactic
    stars — astro data correct), `m24` (IC4715, `*Ass`, "Small Sgr Star
    Cloud" — same object as M24): all safe.

- **Addendum M102 / Dup type (spec gap for parser tasks 4/5):** addendum
  row `M102` has `M=101` and `Type=Dup` (OpenNGC marks it as a
  duplicate of M101/NGC5457, the Pinwheel Galaxy). The binary's slug
  algorithm would generate `m101` from this row, then UPSERT it with
  `object_type='Dup'`, clobbering the correct `G` (galaxy) type already
  written from NGC5457. The parser **must skip rows where `Type='Dup'`**
  (or, equivalently, in the addendum second-pass, skip any row whose
  target slug already has astro metadata populated). This is a parser
  correctness requirement, not just a KEEP_MANUAL_META question; tracked
  as a constraint for Task 4 (CSV parser) and Task 5 (slug rules).
- **Multi-target dedup:** user picks "M42" and "NGC 1976" (same
  object). Front dedupes by slug **after** autocomplete returns
  canonical slugs (NGC 1976 lookup → m42). Backend rejects duplicate
  slugs in the array as 400.
- **`/t` index perf:** 14k rows + filters + LATERAL subquery for
  preview thumbs. Expected <100 ms with the existing indexes. If
  perf degrades, switch preview thumbs to a daily-refreshed
  materialized view. Don't optimize preemptively.
- **Backfill ambiguity:** "M5" matches Messier 5, but a user might
  have meant "M 5" filter wheel position. Free-text matching ignores
  whitespace differences; ambiguous matches logged as warnings,
  human-reviewed before `--apply`. Accept 1–2% false positives —
  the photo's free-text `target` is preserved alongside the join row,
  so original intent is never lost.

---

## Out of scope (deferred)

- **D3** — Aladin Lite WebGL sky-map embed on `/t/<slug>` (trivial
  once RA/Dec are in place).
- **D4** — NASA / ESA / JWST gallery proxy with 24h cache.
- **D5** — Plate solving (Astrometry.net) async worker, populating
  `photo_targets` rows with `source='plate_solve'` and `confidence`.
- **D6** — Cone search (RA/Dec radius). Requires `cube` /
  `earthdistance` Postgres extension and a spatial index.
- **Object descriptions** (Wikipedia extract, AI-generated) — not
  needed for D2 content. The enriched header already conveys context.
- **i18n infrastructure** — `OBJECT_TYPE_LABELS` and
  `CONSTELLATION_LABELS` ship as French-only constants. When the
  project gets i18n, these tables move to the global system.
- **Materialized view for preview thumbs** — only if perf demands it.

---

## References

- OpenNGC repo — https://github.com/mattiaverga/OpenNGC (CC-BY-SA 4.0)
- IAU constellation codes —
  https://www.iau.org/public/themes/constellations/
- Existing schema: `backend/migrations/0010_targets_tags.sql`
- Existing route: `frontend/src/routes/t/[slug]/+page.svelte`
- Existing autocomplete: `backend/src/photos/targets_autocomplete.rs`
- Photographer-showcase discovery (origin of `/t/`):
  `docs/superpowers/specs/2026-05-03-photographer-showcase-design.md`
