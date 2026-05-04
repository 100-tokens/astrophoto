# P3 acceptance — Photographer Showcase Phase 3 (Discovery)

This is the acceptance evidence for the P3 section of the spec at
`docs/superpowers/specs/2026-05-03-photographer-showcase-design.md`
(lines 793–1006) and the implementation plan at
`docs/superpowers/plans/2026-05-04-photographer-showcase-p3-discovery.md`.

P1 / P2 acceptance docs are the precedent for the format.

## Scope summary

P3 surfaces the corpus across photographers. Six new public read
endpoints, six SvelteKit routes, a global `<SearchBar>` in the navbar
with a ⌘K hotkey, and the cross-author tile/grid that extend P2's
`<PhotoTile>` / `<PhotoGrid>` pattern with an author-chip overlay. No
new migrations — schema and indexes shipped in P1 (0010 / 0011 / 0012).

## Backend automated regression

Run from the worktree:

```
cd backend && cargo test --tests
```

| Suite | Tests | What it proves |
|---|---|---|
| `tests/discovery_explore`  | 4 | Newest cross-author order; limit clamp 1..=60; category filter; sort=most-appreciated |
| `tests/discovery_target`   | 2 | Hit returns target meta + photos; 404 unknown slug |
| `tests/discovery_tag`      | 2 | Hit returns tag meta + photos; 404 unknown slug |
| `tests/discovery_equipment`| 3 | Hit returns equipment meta + photos + paired rail; 404 unknown kind; 404 unknown slug |
| `tests/discovery_category` | 2 | Hit filters photos by category; 404 invalid category (off-whitelist) |
| `tests/discovery_search`   | 3 | Targets+users+photos returned for a single query; 400 on empty q; smoke 200 for arbitrary q |
| Pre-existing P1 + P2 suites | — | No regressions; full `cargo test --tests` exit 0 |

The full sweep (`cargo test --tests --test-threads=2`) returned exit 0
during P3 development.

## Frontend automated regression

```
cd frontend && pnpm vitest run
cd frontend && pnpm check
cd frontend && pnpm lint
```

- 11 unit tests pass (formatIntegration + tiptapAllowlist drift,
  pre-existing).
- `pnpm check`: 0 errors, 19 warnings (5 pre-existing + the new
  `state_referenced_locally` patterns; the eslint config has per-file
  overrides for those).
- `pnpm lint`: prettier + eslint clean.
- 3 Playwright spec files are still collected by vitest and reported
  as "failed test files" — they are P1 leftovers (`p1-happy-path.spec.ts`)
  that need an explicit vitest exclude. Pre-existing; not in scope for
  P3 to remove.

## Quality gates

```
just check
```

Exit 0. Covers backend `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings`,
frontend `pnpm check` (svelte-check), `pnpm lint` (prettier + eslint).

## What the chrome-devtools-mcp browser walk would cover

Per the project memory `E2E tooling — chrome-devtools-mcp not Playwright`,
end-to-end acceptance for P3 is meant to be a browser walk recorded here
(mirroring P1 / P2 cadence). The 15-step walk plan is below. **It has
not been driven in the agent session that produced this PR**; the
operator should drive it directly to confirm visual fidelity, then
amend this document with pass/fail per step + diagnostic screenshots.

1. `/explore` renders with photos seeded by the existing dev DB or by `cargo test`.
2. Sort selector (Newest / Most appreciated) re-orders the grid.
3. Time-window selector (24h / 7d / 30d / All) filters the grid.
4. Category chips (DSO / Planetary / Lunar / Solar / Wide-field / Nightscape) filter the grid.
5. Following-only checkbox toggles to the auth-only filter.
6. Click a tile → lightbox opens (shallow routing inherited from P2).
7. Navigate to `/t/m31` (after seeding); confirm `<DiscoveryHeader variant="target">` + grid.
8. Navigate to `/tag/<slug>` if a tag exists.
9. Navigate to `/equip/camera/<canonical>`; confirm "Often paired with" rail renders chips.
10. Navigate to `/c/dso`; confirm category filter applies.
11. Type `andromeda` in the navbar `<SearchBar>`; suggestions appear; click a target → navigates to `/t/...`.
12. Press ⌘K from any page; the search bar focuses.
13. Visit `/search?q=andromeda` directly; three sections render.
14. Empty state: visit `/t/notathing` → 404 page.
15. Empty state: visit `/c/dso` with no DSO photos → "No photos in this category yet."

## Deviations from the plan

- **No drift in TS bindings** — `just types` after Task 9 produced no
  changes (Task 2 already wrote the right shapes), so no separate
  Task 10 commit was needed.
- **`exactOptionalPropertyTypes` everywhere** — the routes use
  `...(category !== undefined ? { category } : {})` spread patterns to
  satisfy strict `tsconfig` rather than passing `undefined` explicitly.
- **`{#key}` block on the filter tuple** in each page's `+page.svelte`
  remounts `<CrossAuthorGrid>` when a filter changes, flushing the
  stale `extraPhotos` `$state` cleanly. Slight presentational nit
  (re-fetches the first page on each filter change rather than
  paginating from cursor zero) but functionally correct.
- **`svelte/valid-compile: 'off'` in eslint** for the new page files
  too — the same intentional `state_referenced_locally` pattern from P2
  recurs in routes that seed local cursor `$state` from `data` props.

## Pre-existing items observed during P3, not addressed here

- `Modal.svelte` a11y warning — pre-existing from P1.
- `frontend/test-results/` Playwright debris — committed during P1's
  acceptance walk and ignored by `.prettierignore` since P2.
- `frontend/tests/e2e/p1-happy-path.spec.ts` Playwright spec — gets
  picked up by vitest and reported as a "failed test file" in `pnpm vitest run`.
  Excluding it from vitest (`vitest.config` `exclude: ['**/tests/e2e/**']`)
  is the right fix; deferred to a chore commit.

## Next phases (not in this spec)

- Plate-solving (writes `photo_targets.source = 'plate_solve'` rows).
- Search v2 (Postgres `tsvector` materialised column + GIN; current v1
  is ILIKE).
- Collections (replaces the "Featured" slot for portfolio curation).
- Subscriptions / billing UI for the tier gate.
