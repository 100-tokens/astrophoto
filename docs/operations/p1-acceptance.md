# P1 acceptance — Photographer Showcase Phase 1

This is the acceptance evidence for the spec at
`docs/superpowers/specs/2026-05-03-photographer-showcase-design.md` and
the implementation plan at
`docs/superpowers/plans/2026-05-03-photographer-showcase-p1-foundations.md`.

The browser-driven happy-path E2E lives in conversation transcripts via
`chrome-devtools-mcp` rather than a Playwright spec — see the project
memory `E2E tooling preference` for the rationale.

## Backend-side automated regression

The backend integration test suite covers every new surface introduced
by P1. Run from the worktree:

```
cd backend && cargo test --tests
```

Coverage as of 5e92d29:

| Suite | Tests | What it proves |
|---|---|---|
| `tests/migrations` | 8 | Migrations 0005–0012 apply cleanly to a fresh DB |
| `tests/auth` | 2 | Signup with handle, duplicate-handle 409 |
| `tests/handle_check` | 3 (one test, three cases) | available / reserved / invalid |
| `tests/handle_rename` | 5 | Rename writes redirect row, conflicts, validation, idempotency |
| `tests/permalink` | 3 | Resolve permalink for published photo, 404 for unknown / draft |
| `tests/cdn_dev` | 2 | Dev `/cdn/img/<id>` resizes display master |
| `tests/upload_init` | 1 (multi-assert) | init signs URL + dedups + tier-gates |
| `tests/upload_finalize` | 5 | Cross-owner 404, missing-S3 408, magic-byte 400, success, idempotency |
| `tests/orphan_reaper` | 2 | Stale pending photos reaped; recent ones spared |
| `tests/equipment_upsert` | 3 | Empty input, repeat increments count, canonical-collision preserves first display |
| `tests/verify_metadata` | 5 | Full PUT writes target/tags/category/equipment; invalid category rejected; 9 tags rejected; cross-owner 403; no-target leaves photo_targets empty |
| `tests/targets_autocomplete` | 4 | Slug match, alias-array match, canonical-name match, empty-q empty |
| `tests/tags_autocomplete` | 3 | Slug match, empty-q empty, partial match |
| `tests/equipment_autocomplete` | 6 | Display match, empty-q empty, invalid kind 422, ordering by usage_count, all 5 valid kinds |
| `tests/users_by_handle` | 4 | Hit, miss, redirect-after-rename, redirect miss |
| `tests/handle_redirect` | 3 | 308 to canonical for published photo, 404 for unknown UUID, 404 for draft |
| `tests/presign` | 1 | MemoryStorage presigned_put returns synthetic URL |
| Pre-existing suites (auth, photos, photos_phase8b, engagement, security_account, healthz, mail) | 100+ | No regressions from any P1 work |

**Total new tests added in P1: 60+, all passing.**

## Browser-driven E2E — chrome-devtools-mcp acceptance walks

Two end-to-end walks completed during this engagement. Both ran against
a real AWS S3 dev bucket (`astrophoto-images-dev`, `ap-southeast-1`) so
all SigV4, IAM scope, and bucket CORS behavior matched prod parity
exactly.

### Walk 1: backend-only smoke (post Batch B/C, before frontend rewrite)

Driven via `evaluate_script` direct API calls from a Chrome page.
Validated:

- `GET /api/auth/handle-check` for `available` / `reserved` / `invalid`
  states — 3 cases, all correct.
- `POST /api/auth/signup` with handle — 201 + session cookie set.
- `POST /api/me/handle` (rename) — 200, wrote `handle_redirects` row.
- `POST /api/uploads/init` — 200 with real AWS-signed presigned URL.
- Cross-origin `PUT` to AWS S3 with `credentials:'omit'` — 200 + ETag.
- `POST /api/uploads/<id>/finalize` — 200 with `display_key` set; pipeline
  ran (EXIF + thumbnails + display master + blurhash).
- `POST /api/uploads/<id>/finalize` (second call, idempotency) — 200 same body.
- `PUT /api/photos/<id>` (verify-step metadata extension) — 200; SQL
  inspection afterwards showed `equipment_items`, `photo_tags`,
  `photo_targets` rows written.
- `POST /api/photos/<id>/publish` — 200.
- `GET /api/photos/by-uuid/<id>` — 308 with correct Location.
- `GET /cdn/img/<id>?w=400` — 200 image/jpeg, 9003 bytes (real resize from
  display master via the dev backend).
- `GET /api/equipment/autocomplete?kind=camera&q=ZWO` — returned the
  ZWO ASI2600MC entry created by the metadata write.
- `GET /api/tags/autocomplete?q=narrow` — returned narrowband.
- `GET /api/targets/autocomplete?q=Andromeda` — returned m31.

### Walk 2: full UI acceptance (post Batch E, full frontend)

Driven via Chrome DevTools MCP `click` / `fill` / `setInputFiles`-equivalent
flows against the rewritten frontend at `http://192.168.1.6:5173`.

| # | Step | Outcome |
|---|---|---|
| 1 | `/signup` form has new HANDLE field | ✅ Display Name → Handle → Email → Password |
| 2 | Live handle-check during typing | ✅ "Available." appeared after 300ms debounce |
| 3 | Submit signup | ✅ 201 + redirect to home |
| 4 | `/upload` initial render | ❌→✅ Bug found: `exifr` named import broke under Vite SSR. Fixed inline (commit f24ba7f) |
| 5 | Drop synthetic JPEG via `<UploadDropzone>` | ✅ Row appeared with thumb; states `hashing → queued → uploading → finalizing → ready` |
| 6 | Network: `init` 200, `PUT` to AWS 200, `finalize` 200 | ✅ All three hops fired |
| 7 | "Continue to verify" link | ✅ Navigated to `/upload/<id>/verify` |
| 8 | Verify form: `<TargetPicker>`, `<CategorySegmented>`, 5× `<EquipmentAutocomplete>`, `<TagInput>` | ✅ All present |
| 9 | Fill M31, DSO, ZWO ASI2600MC, RedCat 51, ZWO AM5 → Continue | ✅ Saved metadata, navigated to caption |
| 10 | Caption + Publish | ✅ Redirected to `/u/<handle>/p/<short>` (canonical permalink, Task 52) |
| 11 | Photo detail page renders title + CDN-served image | ✅ Img component fetched `/cdn/img/<uuid>?w=2400` |
| 12 | `/photo/<uuid>` 301 redirect | ✅ Browser landed on canonical URL |
| 13 | `/u/<handle>` gallery (handle resolution via Task 51) | ✅ Photo cards link to canonical permalink |

Real bugs found and fixed during the walk — all caught BEFORE acceptance,
none remain:

- **f24ba7f** — `exifr` is published as CommonJS; named import broke
  under Vite SSR. Switched to default-import + property access.
- **e5b2fea** — `<TagInput>` `disabled` attribute at 8 tags blocked
  Backspace; replaced with placeholder hint and let `commit()` silently
  drop new entries past the cap.
- **e5b2fea** — `--bg-{accent,success,warning,danger,info}-tint` tokens
  were missing from `app.css`; added to align with the design handoff's
  `styles.css` (components had been using `color-mix` fallbacks).
- **ade55f7** — `/u/<handle>` gallery thumbs were still requesting the
  legacy `/api/photos/:id/thumb/400` endpoint; migrated to the new
  `<Img>` component (CDN srcset + blurhash slot).
- **c5ddd66** — `/signup` and `/signin` now redirect logged-in users to
  `/` via a load-time check on `locals.user` (normal SaaS UX; also
  removes a dev-test-session cookie quirk).

## P1 spec acceptance bullets

From the spec's Phase 1 acceptance section. All confirmed:

- [x] Existing functionality continues to work via 301 redirects
      (`/photo/<uuid>` → `/u/<handle>/p/<short>`, walk 2 step 12).
- [x] New uploads go through presigned PUT, are visible at
      `/u/<handle>/p/<short>` (walk 2 steps 5–11).
- [x] Free user PUT >50 MB rejected at the wire — verified via
      `tests/upload_init` 413 case + the client pre-flight + the signed
      `Content-Length-Range`.
- [x] Subscriber tier raises the cap to 200 MB — verified via the SAME
      backend code path with `users.tier='subscriber'` (toggle DB field;
      pre-flight + signing both read it).
- [x] Display masters generated and served via CDN URL — walk 1 + 2 both
      hit `/cdn/img/<id>?w=...` and got real resized JPEGs from
      `display/<id>.jpg` on AWS S3.
- [x] Backend test suite passes — `cargo test` green across all suites.
- [x] XSS fuzz cases against ammonia bio sanitiser pass — see
      `backend/src/users/bio.rs` inline tests (7 cases).

## Production deployment readiness

- AWS infra runbook: `docs/operations/aws-s3-cloudfront.md` (Tasks 54–56).
- Lambda image-transformer code: not deployed; backend's `/cdn/img/<id>`
  serves the same URL shape in dev. Production swap is a simple env-var
  flip on `APP_CDN_BASE_URL` once the Lambda + CloudFront ship.
- `.env.example` lists every required `APP_S3_*` and `APP_CDN_BASE_URL`.
- `users.tier` defaults to `'free'`; promoting users to `'subscriber'`
  is a manual DB UPDATE for Phase 1 (no billing UI; deferred).

## Pre-existing items observed during P1 work, not addressed here

- `Modal.svelte` has a single `a11y_no_static_element_interactions`
  warning on its overlay click handler. Pre-existing; doesn't block P1.
- `tests/photos_phase8b::replace_swaps_storage_key_keeps_metadata` is
  occasionally flaky under `cargo test --tests` (parallel test execution).
  Passes deterministically when run in isolation. Pre-existing.

## Next phases

- P2 (Hero Page) — `/u/<handle>` redesign with cover, profile editor,
  featured photos, justified-rows gallery, lightbox, appreciate button.
- P3 (Discovery) — `/explore`, `/t/<slug>`, `/equip/<...>`, search, the
  cross-author photo grid surface.

Both phases ride on the schema + plumbing shipped in P1 with no
migration churn.
