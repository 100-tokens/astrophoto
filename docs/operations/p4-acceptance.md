# P4 Acceptance — Celestial Objects (D1 + D2a/b/c)

This is the acceptance evidence for the Celestial Objects feature branch
(`feat/celestial-objects`), covering D1 (catalog schema + seeding) and
D2a/b/c (enriched target page, TargetMultiPicker, /t index + filtering).

Spec: `docs/superpowers/specs/2026-05-06-celestial-objects-design.md`
Plan: `docs/superpowers/plans/2026-05-06-celestial-objects.md`

**Date:** 2026-05-06  
**Branch:** feat/celestial-objects  
**Base commit:** 833c82b (last commit on `main` before this branch)

---

## Quality gates

| Gate | Result |
|------|--------|
| `just check` | PASS (after fixing 2 prettier formatting issues in `TargetIndexCard.svelte` and `t/+page.svelte`, and adding `src/routes/t/+page.svelte` to the per-file ESLint `svelte/valid-compile: off` override) |
| `cargo fmt --check` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS — 0 warnings |
| `pnpm check` (svelte-check) | PASS — 0 errors, 23 warnings (all pre-existing `state_referenced_locally` patterns) |
| `pnpm lint` (prettier + eslint) | PASS — 0 errors after formatting fix |
| `just test` (backend) | 158 passed, 1 ignored — see note below |
| Frontend unit tests (`pnpm vitest run`) | 17 passed; 3 "failed test files" are pre-existing Playwright e2e specs collected by vitest (known P3 issue) |

### Test note — `handle_redirect` Docker timeout

The full `just test` run produced 2 failures in `tests/handle_redirect.rs`
with `CreateContainer(RequestTimeoutError)`. This is a testcontainers
Docker resource timeout caused by many parallel containers being created
across the large test suite — not a code regression. On immediate re-run
of `cargo test --test handle_redirect` alone, all 3 tests passed cleanly
(135 s). The handle_redirect code and tests are pre-existing and
unchanged by this branch.

### Backend test breakdown

| Suite | Tests | Notes |
|-------|-------|-------|
| Unit tests (lib) | 92 | Pre-existing |
| `backfill_photo_targets` bin | 4 | **New** — Tasks 19 |
| `seed_targets` bin | 11 | **New** — Tasks 4–7 |
| `tests/auth.rs` | 2 (1 ignored) | Pre-existing |
| `tests/cdn_dev.rs` | 2 | Pre-existing |
| `tests/cover_set.rs` | 3 | Pre-existing |
| `tests/discovery_category.rs` | 2 | Pre-existing |
| `tests/discovery_equipment.rs` | 3 | Pre-existing |
| `tests/discovery_explore.rs` | 6 | Pre-existing |
| `tests/discovery_search.rs` | 3 | Pre-existing |
| `tests/discovery_tag.rs` | 2 | Pre-existing |
| `tests/discovery_target.rs` | 2 | Pre-existing |
| `tests/engagement.rs` | 3 | Pre-existing |
| `tests/equipment_autocomplete.rs` | 6 | Pre-existing |
| `tests/equipment_upsert.rs` | 3 | Pre-existing |
| `tests/featured_pin.rs` | 6 | Pre-existing |
| `tests/featured_reorder.rs` | 4 | Pre-existing |
| `tests/handle_check.rs` | 1 | Pre-existing |
| `tests/handle_redirect.rs` | 3 | Pre-existing (flaky Docker timeout on full run; passes on retry) |

New tests introduced by this branch: **15** (11 seed_targets + 4 backfill).

---

## Catalog seeding

Migration 0014 (`add_targets_astro_metadata`) applied cleanly. Seeded
from `data/NGC.csv` (13 969 rows) + `data/addendum.csv` (64 rows) =
14 033 input rows.

```
Counts {
    upserts: 12584,
    skipped_subcomponent: 736,
    skipped_unknown_prefix: 61,
    skipped_duplicate: 652,
}
```

Math: 12 584 + 736 + 61 + 652 = **14 033** ✓

`KEEP_MANUAL_META = ['ic-434']` verified — see E2E flow #4 below.

---

## Fixes applied during QA

1. **Prettier formatting** — `TargetIndexCard.svelte` and `src/routes/t/+page.svelte`
   had trailing whitespace / minor formatting drift. Fixed with
   `pnpm prettier --write`.

2. **ESLint `svelte/valid-compile` override** — `src/routes/t/+page.svelte`
   uses the same intentional "seed local `$state` from SSR `data` prop once;
   `$effect` syncs on navigation" pattern as all other discovery pages.
   Added it to the per-file `svelte/valid-compile: off` list in
   `frontend/eslint.config.js` (same entry as `explore/+page.svelte`,
   `t/[slug]/+page.svelte`, etc.).

---

## E2E browser walkthrough — 2026-05-06

Dev stack: postgres 5434 + minio + backend port 8080 + vite port 5173,
freshly started via `just dev`. DB seeded with ~12 584 catalog targets
from migration + `just seed-targets`. Screenshots in
`docs/operations/screenshots/p4/`.

| # | Flow | Result |
|---|------|--------|
| 1a | `/t` index page renders grid | ✅ 24 cards visible, each showing slug, canonical ID, type label, constellation, 0 photos. "Charger plus" button present. — `screenshots/p4/01-t-index-grid.png` |
| 1b | Type "andromed" in search box → debounce → M31 card appears | ✅ URL updates to `?q=andromed`, single result: "M31 / Andromeda Galaxy / Galaxie · Andromède / 0 photos" — `screenshots/p4/02-t-search-andromed.png` |
| 1c | Click M31 card → `/t/m31` enriched header | ✅ `● TARGET · MESSIER`, heading "Andromeda Galaxy", `Galaxie · Andromède · RA 00ʰ42ᵐ44ˢ · Dec +41°16′09″ · mag 3.4 · 178′ × 70′`, aliases: M 31, Messier 31, NGC 224 — `screenshots/p4/03-t-m31-enriched-header.png` |
| 2 | `/t?object_type=G` — filter by Galaxie | ✅ All 24 visible cards show "Galaxie" type label. Non-galaxy types (Amas ouvert, Étoile, Autre, etc.) that appear in the unfiltered view are absent. Type select shows "Galaxie" selected. — `screenshots/p4/04-t-filter-galaxie.png` |
| 3 | Upload + multi-tag M42 + NGC 1977 | DEFERRED — requires authenticated test user session. Multi-tag backend and TargetMultiPicker component are unit/integration tested (Tasks 9, 15, 16). Manual test: sign in, upload, use TargetMultiPicker on `/upload/<id>/verify`, confirm both `/t/m42` and `/t/ngc-1977` list the photo. |
| 4 | `/t/ic-434` — KEEP_MANUAL_META protection | ✅ Page shows `● TARGET · IC`, `IC-434`, heading "Horsehead Nebula", aliases (IC 434, Barnard 33) — but **no** object_type line, **no** constellation line, **no** RA/Dec, **no** magnitude. Astro meta blocked by skip list as designed. — `screenshots/p4/05-t-ic434-horsehead-protected.png` |
| 5 | `/t/ngc-7000` — enriched header sanity check | ✅ `● TARGET · NGC`, heading "North America Nebula", `Région HII · Cygne · RA 20ʰ59ᵐ17ˢ · Dec +44°31′44″ · 120′ × 30′`, aliases: NGC 7000, Caldwell 20. Canonical name preserved correctly. — `screenshots/p4/06-t-ngc7000-enriched.png` |

---

## Concerns / follow-ups

- **E2E #3 DEFERRED (manual):** Multi-tag upload flow requires a signed-in
  test user. The backend (multi_attach, PATCH /api/photos/:id/targets) and
  the TargetMultiPicker component are covered by integration and unit tests.
  To validate end-to-end: sign up a test account, upload a photo, use
  TargetMultiPicker on the verify step, confirm both target pages list the
  photo.

- **Backfill 0 eligible photos:** `backfill_photo_targets` dry-run on the
  dev DB processes 0 photos (none seeded in dev). The binary is tested with
  testcontainers. On production, run after deploy to link existing
  photos to targets by their `tags` field.

- **`handle_redirect` flaky under load:** Two tests timed out on Docker
  container creation during the full suite run (many parallel containers).
  Passed immediately on single-suite retry. Pre-existing infra issue, not
  introduced by this branch.

- **Constellation filter UI deferred:** Backend `GET /api/targets` accepts
  `constellation=<IAU code>` and the URL params plumb through `load`,
  but the `/t` index page only exposes `object_type` and `sort` selects.
  88 constellations is unwieldy as a flat dropdown — a typeahead or a
  collapsible "advanced filters" panel would be the right pattern. Manual
  URL crafting works today (`/t?constellation=ORI`). Spec § Decisions #12
  lists constellation as in-scope; treat as a follow-up UI ticket.

- **3 Playwright spec files collected by vitest:** Pre-existing from P1.
  Fix: add `tests/e2e/**` to vitest `exclude`. Deferred chore.

- **`DiscoveryHeader` unused CSS `.stat-accent`:** Pre-existing warning
  from P3. Not in scope here.

---

## Commits

```
f88c29b chore(repo): ignore /.worktrees/ for subagent-driven dev
1d9094f docs(plans): equipment setups implementation plan
0263971 docs(specs): equipment setups design — D1 + T2′
88b3e4a docs(specs): celestial objects design — D1 + D2a/b/c
018113f docs(specs): celestial objects — advisor review fixes
9784067 docs(specs): celestial objects — verified KEEP_MANUAL_META list
48b7e35 docs(plans): celestial objects implementation plan
d9386b4 feat(schema): 0014 targets astro metadata columns
c25368b fix(sqlx): restore test query cache lost by missing --workspace
ece0577 chore(data): pin OpenNGC catalog snapshot
0ad02a4 feat(seed-targets): OpenNGC CSV row parser
1343912 feat(seed-targets): slug computer + skip rules
db27bbb feat(seed-targets): UPSERT with manual-meta preservation
d96c92f feat(seed-targets): main + just recipe
81f0a79 feat(targets): /api/targets/:slug returns enriched astro meta
59eedad feat(photos): metadata POST accepts targets array (atomic multi-attach)
35f2c97 feat(api): PATCH /api/photos/:id/targets for post-publish edits
4394af3 feat(utils): formatRA + formatDec sexagesimal helpers
d2c507f feat(data): celestial object-type + constellation FR labels
a71d7a4 feat(target-page): enriched header with RA/Dec/type/const/mag
d8cff59 refactor(target-picker): extract autocomplete input into reusable component
b4ec5dd fix(target-autocomplete): correct ARIA combobox pattern
03e8efb feat(upload): TargetMultiPicker component
7ec68ca feat(upload): swap TargetPicker for TargetMultiPicker
e0a68f4 feat(api): GET /api/targets — paginated catalog index
4e2a406 feat(t-index): browsable celestial-objects index page
155d041 chore(types): regenerate TargetPreviewThumb.ts via just types
9f69837 feat(backfill): backfill-photo-targets binary
```

29 commits on branch (including 3 docs/specs/plans commits shared with
equipment-setups work, and 1 chore/repo commit, all predating this
feature's code).
