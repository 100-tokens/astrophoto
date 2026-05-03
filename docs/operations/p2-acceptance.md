# P2 acceptance — Photographer Showcase Phase 2 (Hero Page)

This is the acceptance evidence for the P2 section of the spec at
`docs/superpowers/specs/2026-05-03-photographer-showcase-design.md`
(lines 542–789) and the implementation plan at
`docs/superpowers/plans/2026-05-03-photographer-showcase-p2-hero-page.md`.

P1's `p1-acceptance.md` is the precedent for the format used here.

## Scope summary

P2 rebuilt `/u/<handle>` into a polished public profile (cover,
identity, sanitised rich-text bio, equipment strip, location badge,
stats row, 6-slot featured row, justified-rows gallery, deep-linked
lightbox) and added the profile editor (Tiptap), cover picker, and
featured pin/unpin/reorder controls. The schema landed in P1; P2
shipped the remaining backend writers and the frontend surface.

## Backend automated regression

Run from the worktree:

```
cd backend && cargo test --tests
```

| Suite | Tests | What it proves |
|---|---|---|
| `tests/profile_extended` | 5 | GET full shape, PATCH writes tagline+bio with sanitisation, equipment+location, social_links validation (host whitelist, success), explicit-null clears |
| `tests/cover_set` | 3 | Sets `users.cover_photo_id`; cross-owner returns 404; `photo_id: null` clears |
| `tests/featured_pin` | 6 | Pin assigns 1 then 2; idempotent; 409 when full; cross-owner 404; unpin compacts; unpin idempotent |
| `tests/featured_reorder` | 4 | Reorder rewrites positions in one tx; 400 on unpinned id, duplicate id, > 6 ids |
| `tests/public_profile` | 2 | Aggregator returns full shape (handle, equipment, location, social_links, stats, featured); 404 unknown handle |
| `tests/photos_feed` | 4 | Newest-first cursor pagination, 404 unknown handle, limit clamped 1..=60, sort=popular orders by appreciations_count |
| `users::bio` (lib unit) | 8 | Existing 7 sanitiser cases + new drift test (Rust ALLOWED_TAGS == JSON `tags`) |
| `users::social_links` (lib unit) | 8 | Validator: canonical/x.com twitter, wrong host, javascript: scheme, > 6 links, duplicate platform, website/mastodon any-host |

Also exercised at the DB layer (no new migrations — P1 shipped the
columns this phase needs):

- `users.{tagline, bio_html, cover_photo_id, equipment_*, location_text, bortle_class, sqm, social_links}` (migration 0008)
- `photos.{featured_at, featured_position, category}` + `photos_featured_per_user_uidx` partial unique + `photos_featured_pair_chk` check constraint (migration 0009)
- `photos.appreciations_count` denormalised counter (migration 0011)

The pin/unpin/reorder transaction logic stages writes as
`featured_at = null, featured_position = null` BEFORE writing the
target position so neither the partial unique index nor the pair-NULL
check constraint is tripped mid-update.

## Frontend automated regression

```
cd frontend && pnpm vitest run
cd frontend && pnpm check
```

| Suite | Tests | What it proves |
|---|---|---|
| `format/integration.test.ts` | 6 | `formatIntegration` renders zero/negative/NaN as em-dash, hours, hours+minutes, minutes-only |
| `editor/tiptapAllowlist.test.ts` | 1 | The frontend `ALLOWED_HTML_TAGS` const matches `backend/data/bio-allowed-tags.json` exactly |

`pnpm check`: 0 errors, 14 warnings. Warnings:
- 1 pre-existing `Modal.svelte` a11y warning (inherited from P1).
- 13 `state_referenced_locally` warnings on the editor section
  components and `FeaturedRow`/`PhotoGrid` — the "seed local `$state`
  from prop once" pattern is intentional (the components own local
  edits between explicit save calls). The eslint rule
  `svelte/valid-compile` is disabled for those specific files.

## Quality gates

```
just check
```

Exit 0. Covers: backend `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
frontend `pnpm check` (svelte-check), frontend `pnpm lint` (prettier + eslint).

## Build

```
cd frontend && pnpm build
```

Reported clean by the implementer agents during the run; the operator
should re-run before deploying staging.

## What was NOT walked end-to-end in a browser

Per the project memory `E2E tooling — chrome-devtools-mcp not Playwright`,
P2 acceptance is meant to include a 24-step `chrome-devtools-mcp`
browser walk recorded in this file (mirroring the format of P1's
walks). **That walk has NOT been run in the agent session that
produced this PR.** The 24 steps remain in the plan
(`Task 36`) for the operator to drive directly. They are:

1. New-tab `/signup`, complete signup with handle `marie2`.
2. Navigate to `/u/marie2`. Confirm visitor-as-owner empty-state hooks render: "Pick a cover from your gallery →", "Add a tagline", "Tell visitors about your astrophotography", "Add the gear behind your shots", "Where do you observe from?", featured "SLOT 01" with `[+ Pin a photo]`.
3. Click `Edit profile`. Fill display name, tagline, bio (use bold + italic + a link). Save by tabbing away.
4. Verify the fields persist by reloading the page.
5. Add equipment cells (RedCat 51 / ASI2600MC / ZWO AM5 / L-Pro / ASI120MM). Confirm the strip renders the populated cells and hides empty ones.
6. Set location, Bortle 6, SQM 19.8. Confirm the badge renders.
7. Add a Twitter and an Instagram link. Confirm icons render.
8. Open in a logged-out tab and confirm visitor view: empty-state prompts hidden; populated fields render.
9. Upload three photos via `/upload`; publish them.
10. Reload the hero page. Confirm `frames=3` and the photos appear in the gallery in justified rows.
11. Open the cover picker, pick photo #1, confirm it renders as the cover with the `● COVER · <target>` credit.
12. Pin photos 1, 2, 3 via the featured slot 01 affordance. Confirm slots 04..06 still render placeholders.
13. Reorder featured photo 1 to slot 3 via the ←/→ buttons. Reload; confirm the new order.
14. Unpin photo 2 via the ✕ button. Confirm slots compact (now 1, 3).
15. Click a gallery tile. Confirm the lightbox overlays the page (URL changes; gallery still visible behind).
16. Press `→` then `←` in the lightbox; confirm prev/next swap the image.
17. Press `i`; confirm the EXIF panel collapses, then reopens.
18. Press `a`; confirm the appreciate count increments.
19. Press `Esc`; confirm the lightbox closes and the URL returns to `/u/marie2`.
20. Reload at `/u/marie2/p/<short>` directly; confirm the full photo-detail page renders (no lightbox overlay).
21. Inspect a sample bio with a `<script>` tag pasted via DevTools `evaluate_script` PATCH; confirm the saved value comes back without `<script>` (sanitiser).
22. Verify a malformed `social_links` POST (`{"platform":"twitter","url":"https://evil.example/marie"}`) returns 400.
23. Verify an attempt to pin a 7th photo returns 409.
24. Verify a cross-owner pin attempt returns 404.

## Deviations from the plan

- **Featured drag-reorder** — the plan specified `@neodrag/svelte`
  drag-and-drop reorder; the shipped UI uses ←/→ button controls
  alongside the unpin ✕. The drag UX is deferred as a polish pass; the
  underlying `reorderFeatured` API is already exercised by the
  buttons, so adding drag later is a presentational change with no
  schema/API churn.
- **`@neodrag/svelte`** is in `package.json` (added in Task 1) but not
  yet imported anywhere. Leaving it in for the future polish pass.
- **`bind:` getter/setter pair syntax** — the plan used Svelte 5's
  `bind:value={() => x, (v) => …}` form for editor sections. That
  syntax has compatibility issues in the current Svelte version; the
  shipped code uses a value+`onChange`/`onCommit` callback pattern
  instead. Behaviourally identical.
- **TS regen** is via `cargo run --bin gen-types` (the existing project
  tool); the plan's `cargo test --features ts-rs/no-serde-warnings export_bindings`
  formulation was speculative. The actual `just types` recipe runs the
  binary and passes `prettier --write` over the result.

## Pre-existing items observed during P2, not addressed here

- `Modal.svelte` a11y warning — pre-existing from P1.
- `frontend/test-results/` — Playwright debris committed during P1's
  acceptance walk; added to `.prettierignore` so `just check` doesn't
  trip on it. Worth deleting in a future cleanup commit.
- 13 `state_referenced_locally` Svelte compiler warnings on the
  editor section components — intentional pattern; eslint's
  `svelte/valid-compile` disabled for those files only.

## Next phase

P3 (Discovery) — `/explore`, `/t/<slug>`, `/equip/<...>`, search, the
cross-author photo grid surface — rides on the schema + plumbing this
PR shipped (taxonomy from P1; appreciations counter; gallery feed
shape; public-profile aggregator). No new migrations expected.
