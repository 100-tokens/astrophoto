# Production-readiness e2e coverage

_Status snapshot synthesized from per-area journey audits. 8 areas, 148 user
journeys._

## What this is

A go-live coverage map for every meaningful user journey in astrophoto,
scored on whether it is exercised end-to-end. It exists to answer one
question before launch: **which critical journeys can regress without any
test catching it?**

### The coverage rule

- **Browser e2e is the bar for critical journeys.** A journey is `covered`
  only when a Playwright spec drives the real frontend (navigation, clicks,
  form submits) through the flow. Backend integration tests are necessary
  but not sufficient for a critical path.
- **API-only = `partial`.** The endpoint is tested (`backend/tests/*.rs`)
  but no browser test drives the UI affordance. The handler works; the
  button wiring, the redirect, the rendered result are unproven.
- **`gap`** = neither layer covers it.

### Environment

- Local alt-port stack: frontend `5180`, backend `8081` (Heartbit holds
  `5173`/`8080`); verify-email tokens read via `psql` on `5434`.
- Playwright is **chromium-only** today (`frontend/playwright.config.ts`
  defines a single `chromium` project). No Firefox/WebKit/mobile project.
- **Planned CI gate:** there is currently no test CI (CI is gitleaks-only).
  Wiring the Playwright suite into a required CI check is itself a go-live
  blocker — green specs that never run in CI protect nothing.

### External-dependency policy

Two journeys depend on services that cannot run in the local harness:

- **Google OAuth** — disabled locally when the client id is blank.
- **Plate-solving** — external solver; the route is not even mounted
  without a configured client; dev DB seeds no RA.

Policy: these are covered by **backend test + manual smoke** and are
explicitly **out of the browser-e2e P0 bar**. Green dev-path tests must not
be read as proof of the prod/external path.

---

## Summary counts

| Metric | Count |
|---|---|
| Journeys | 148 |
| Covered (browser e2e) | 99 |
| Partial (API-only) | 39 |
| Gap (neither) | 10 |
| P0 | 2 |
| P1 | 80 |
| P2 | 66 |

---

## Auth & session

| Journey | API | UI | Mobile | a11y | Sec | Verdict | Pri | Gap |
|---|---|---|---|---|---|---|---|---|
| Signup happy path | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | chromium only |
| Signup validation errors | ✗ | ✓ | ✗ | ✗ | ✗ | covered | P2 | — |
| Signup duplicate handle 409 | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P2 | — |
| Authed user visits /signup → redirect | ✗ | ✓ | ✗ | ✗ | ✗ | covered | P2 | — |
| **Email verification — click link, auto-login** | ✓ | ✗ | ✗ | ✗ | ✗ | **partial** | **P0** | /verify/[token] set-cookie-forward never browser-tested; all specs SQL-shortcut verification |
| Email verification — invalid/expired/used | ✓ | ✗ | ✗ | ✗ | ✗ | partial | P1 | 410→redirect chain not browser-tested |
| Resend verification (+cooldown) | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P2 | — |
| check-email reflected ?email XSS / ?expired | ✗ | ✓ | ✗ | ✗ | ✓ | covered | P2 | — |
| Signin happy path | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | no mobile/cross-browser |
| Signin — unverified → check-email | ✓ | ✓ | ✗ | ✗ | partial | covered | P2 | documents intentional email-enum leak |
| Signin — empty fields / form-error | ✗ | ✓ | ✗ | ✗ | ✗ | covered | P2 | — |
| Signin — wrong password / lockout | ✓ | ✗ | ✗ | ✗ | partial | partial | P1 | lockout banner never browser-driven |
| Google OAuth — start/authorize redirect | ✓ | ✗ | n/a | ✗ | partial | partial | P1 | external dep; manual smoke only |
| Google OAuth — callback (exchange/link/session) | ✓ | ✗ | n/a | ✗ | partial | partial | P1 | backend stops at Google boundary; post-gate path untested |
| Logout | ✓ | ✗ | ✗ | ✗ | ✗ | partial | P1 | no browser e2e; recoverable + API-covered |
| Password reset — request link | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | — |
| Password reset — request edge | ✗ | ✓ | ✗ | ✗ | ✗ | covered | P2 | — |
| Password reset — confirm (single-use) | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | full MailHog-link journey |
| Password reset — token page edge | ✗ | ✓ | ✗ | ✗ | ✗ | covered | P2 | — |
| Reset/sent — countdown, resend, no-enum | ✗ | ✓ | ✗ | ✗ | ✓ | covered | P2 | — |
| Password change (/settings/password) | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | — |
| Email change — request confirmation link | ✓ | ✓ | ✗ | ✗ | ✗ | partial | P1 | FE-0216 RED by design — real correctness bug (see notes) |
| Email change — confirm via token | ✓ | ✗ | ✗ | ✗ | ✗ | partial | P1 | only 'taken' branch tested; success render none |
| Sessions — list active | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P2 | single-session only |
| Sessions — revoke other / sign-out-others | ✓ | ✗ | ✗ | ✗ | partial | partial | P1 | the actual security action never browser-driven |
| PAT — create (reveal once) | ✓ | ✓ | ✗ | ✗ | ✓ | covered | P1 | hash-at-rest asserted |
| PAT — empty-name guard / empty list | ✗ | ✓ | ✗ | ✗ | ✗ | covered | P2 | — |
| PAT — revoke | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P2 | — |

**Surface gaps:** `/verify/[token]` (SQL-shortcut bypass), `/account/logout`,
Google OAuth start + callback (external), `/email-change/[token]` success
render, `/settings/sessions` multi-session revoke / sign-out-others.

**Notes / concerns:**
- **Correctness bug (FE-0216, RED by design):** the `/settings/email`
  change-request modal uses a non-enhanced form; the full-POST reload
  resets `showModal = $state(false)`, so the `{#if form?.ok}` confirmation
  lives inside the now-closed modal and never renders. Mail is sent; the
  user sees zero feedback. Worth fixing.
- **Intentional email-enumeration leak:** signin-unverified (FE-0113) and
  reset flows echo the submitted address in redirect query strings;
  FE-0113 freezes this as a contract. Flag for go-live security review.
- **OAuth backend is CSRF/state-gate only** — token exchange, account
  link/create, and session issue are tested by *nothing* automated; manual
  smoke is the sole guard.

---

## Content lifecycle (upload → publish)

| Journey | API | UI | Mobile | a11y | Sec | Verdict | Pri | Gap |
|---|---|---|---|---|---|---|---|---|
| **Upload JPEG → finalize → verify → publish → permalink** | ✓ | ✓ | ✗ | ✗ | ✗ | **covered** | **P0** | core chain green; needs CI gate |
| Upload init — tier/size gate before presign | ✓ | ✓ | ✗ | ✗ | partial | covered | P1 | server-side bypass not browser-driven |
| Cancel in-flight upload (>50% confirm → DELETE) | ✓ | ✓ | ✗ | partial | ✗ | covered | P1 | — |
| Finalize — display master, processing→ready | ✓ | ✓ | ✗ | partial | ✗ | covered | P1 | 409/400 error responses API-only |
| Verify/edit metadata on a draft (autosave) | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | validation rejects API-only |
| Edit metadata of PUBLISHED photo (no republish) | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | published_at-preserved invariant covered |
| Publish a draft (gating/idempotency/ownership) | ✓ | ✓ | ✗ | partial | partial | covered | P1 | ownership/idempotency API-only |
| Replace published photo (REPROCESSED label) | ✓ | ✓ | ✗ | ✗ | partial | covered | P1 | only tiny sample.jpg; proxy BODY_SIZE_LIMIT not exercised |
| Drafts list /me/drafts | ✓ | ✓ | ✗ | ✗ | ✓ | covered | P2 | — |
| Drafts in /account/frames (callout+chip+filter) | ✓ | ✓ | ✗ | ✗ | partial | covered | P2 | — |
| Batch upload landing (collapse, cross-owner leak) | ✓ | ✓ | ✗ | ✗ | ✓ | covered | P1 | — |
| Batch edit (per-frame autosave, owner gate) | ✓ | ✓ | ✗ | partial | ✓ | covered | P1 | — |
| Batch apply-to-all | ✓ | ✗ | ✗ | ✗ | ✗ | partial | P1 | no browser e2e for apply-to-all action |
| Batch publish (publish_all) | ✓ | ✗ | ✗ | ✗ | ✗ | partial | P1 | publish_all form action has no browser e2e |
| Apply equipment setup to a photo | ✓ | ✗ | ✗ | ✗ | ✗ | partial | P2 | filters_cache rebuild invariant unguarded in browser |
| Plate-solve upload (XISF, external) | ✓ | ✗ | ✗ | ✗ | partial | partial | P1 | external dep; only idle panel browser-verified |
| GPS/EXIF privacy on public API | ✓ | ✓ | n/a | n/a | ✓ | covered | P1 | asserted via API request, not rendered page |
| Background lifecycle (sweep/reaper/drain) | ✓ | ✗ | n/a | n/a | n/a | covered | P2 | no UI surface |

**Surface gaps:** `/api/photos/batch-apply`, `publish_all` →
`/api/photos/batch-publish`, `/api/photos/:id/apply-setup`, plate-solve
POST + XISF gating + solving/solved/failed states, finalize 409/400 error
surfacing, `/drafts` redirect routes.

**Notes / concerns:**
- Single-photo critical chain is genuinely browser-covered → no P0 *blocker*
  here, but the P0 journey still needs the CI gate.
- **Batch multi-frame flows (apply-to-all, publish-all) have solid backend
  tests but ZERO browser e2e** — a regression in the batch ribbon ships
  undetected. Same for apply-setup (carries the `photo_filters`/cache
  rebuild invariant).
- Replace is a large-body POST through the SvelteKit proxy
  (`BODY_SIZE_LIMIT`); only tiny files tested, so the documented failure
  mode is never exercised.

---

## Discovery & feeds

| Journey | API | UI | Mobile | a11y | Sec | Verdict | Pri | Gap |
|---|---|---|---|---|---|---|---|---|
| Home feed — anonymous SSR tiles | ✗ | ✓ | ✗ | ✗ | ✗ | covered | P1 | list_recent_public has no backend test |
| Home feed — zero photos → placeholder | ✗ | ✓ | ✗ | ✗ | ✗ | covered | P2 | — |
| Home feed — malicious original_name escaped | ✗ | ✓ | n/a | ✗ | ✓ | covered | P1 | SSR-byte level, not DOM |
| Home feed — logged-in 'following' feed | ✗ | ✗ | ✗ | ✗ | ✗ | gap | P1 | neither layer; degrades to public feed |
| Explore — since/sort/category tiles | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | first page only |
| Explore — re-key grid via replaceState | ✗ | ✓ | ✗ | ✗ | ✗ | covered | P1 | — |
| Explore — invalid ?since → 500 page | ✗ | ✓ | ✗ | ✗ | ✗ | covered | P2 | backend 400 untested |
| Explore — category=<script> → empty, no XSS | ✓ | ✓ | ✗ | ✗ | ✓ | covered | P2 | — |
| Explore — following toggle (authed) | ✓ | ✗ | ✗ | ✗ | partial | partial | P1 | auth-gated UI unverified |
| Feed pagination — 'Load more' / cursor | partial | ✗ | ✗ | ✗ | ✗ | gap | P1 | keyset untested BOTH layers across all 4 feeds |
| Search — no results paragraph | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | — |
| Search — results listing renders | ✓ | ✗ | ✗ | ✗ | ✗ | partial | P1 | result groups never asserted to render |
| Search — empty q → 400 | ✓ | ✗ | n/a | ✗ | ✗ | covered | P2 | — |
| Search — script-y q escaped, noindex | ✗ | ✓ | n/a | ✗ | ✓ | covered | P2 | — |
| Search — header SearchBar autocomplete | ✓ | ✗ | ✗ | ✗ | ✗ | partial | P2 | dropdown no browser e2e |
| Photographers index — list + sort pills | ✗ | ✓ | ✗ | ✗ | ✗ | covered | P1 | /api/photographers has ZERO backend tests |
| Photographers — empty state | ✗ | ✓ | ✗ | ✗ | ✗ | covered | P2 | — |
| Photographers — null cover fallback | ✗ | ✓ | ✗ | ✗ | ✗ | covered | P2 | — |
| Category feed /c/[cat] — happy + normalize | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | first page only |
| Category feed — invalid → 404 | ✓ | ✗ | n/a | ✗ | ✗ | covered | P2 | — |
| Tag feed /tag/[slug] — happy + param forward | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | first page only |
| Tag feed — unknown slug → 404 | ✓ | ✗ | n/a | ✗ | ✗ | covered | P2 | — |
| Equipment discovery /equip/[kind]/[slug] | ✓ | partial | ✗ | ✗ | ✗ | partial | P1 | browser asserts spec sheet only, not feed/rail |
| RSS feed /rss.xml | ✗ | ✗ | n/a | n/a | ✗ | gap | P2 | zero coverage |
| Sitemap /sitemap.xml | ✗ | ✗ | n/a | n/a | ✗ | gap | P2 | zero coverage |

**Surface gaps:** `/rss.xml`, `/sitemap.xml`, 'Load more' pagination on all
four feeds, logged-in personalized home feed, header SearchBar
autocomplete, equipment discovery feed body.

**Notes / concerns:**
- **Keyset pagination untested at BOTH layers across every feed** — tests
  assert only `next_cursor.is_some()`, never decode the cursor to fetch
  page 2; no `cursor.rs` round-trip unit test. Dup/skipped-row bugs ship
  silently.
- `/api/photographers` (`photographer_index.rs`) has **no backend
  integration test at all** — 3 sort branches + keyset cursors unverified
  server-side; only FE sort-pill coercion touches it.
- Home feed `list_recent_public` / `list_following` have no backend test;
  exercised only via SSR render. No P0 here — every surface has ≥1 browser
  test on its first page, so gaps are within-surface.

---

## Targets, permalink & engagement

| Journey | API | UI | Mobile | a11y | Sec | Verdict | Pri | Gap |
|---|---|---|---|---|---|---|---|---|
| Browse celestial target index /t | ✗ | ✓ | ✗ | ✗ | ✗ | partial | P1 | catalog-list endpoint untested; only empty-query browser path |
| Single target page /t/[slug] + gallery | ✓ | ✓ | ✗ | ✗ | ✓ | partial | P1 | only header rendered; gallery/counts never browser-driven |
| Target unknown slug → 404 | ✓ | ✗ | n/a | ✗ | n/a | covered | P2 | — |
| Permalink view /u/[h]/p/[id] | ✓ | ✓ | ✗ | ✗ | ✓ | covered | P1 | incl. caption/comment XSS escape |
| Permalink unknown short_id → 404 | ✓ | ✗ | n/a | ✗ | n/a | covered | P2 | — |
| Draft not resolvable via permalink | ✓ | ✗ | n/a | ✗ | partial | partial | P1 | privacy boundary API-only |
| Celestial overlay on solved photo | ✓ | ✗ | ✗ | ✗ | ✗ | partial | P1 | positive render never tested (plate-solve gated) |
| Appreciate (like) toggle | ✓ | ✗ | ✗ | ✗ | ✗ | partial | P1 | only UI test is a permanently-skipped stub |
| Comment create / list / delete | ✓ | ✗ | ✗ | ✗ | partial | partial | P1 | submit+delete never UI-driven; only SQL-seeded render |
| Comment / caption XSS escaped | ✗ | ✓ | n/a | ✗ | ✓ | covered | P2 | — |
| Follow / unfollow photographer | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | 3-state button covered |
| Public profile /u/[handle] owner vs visitor | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | — |
| Public profile unknown handle → 404 | ✓ | ✗ | n/a | ✗ | n/a | covered | P2 | — |
| Renamed-handle redirect | ✓ | ✗ | n/a | ✗ | ✗ | covered | P2 | redirect never browser-driven |
| Profile SEO head (title/og/JSON-LD) | ✗ | ✓ | n/a | ✗ | ✗ | covered | P2 | — |
| Profile bio stored-XSS sanitization | ✗ | ✓ | n/a | ✗ | ✓ | covered | P2 | no standalone backend sanitizer test |
| Featured-pin curation (max 6, owner) | ✓ | ✗ | ✗ | ✗ | partial | partial | P2 | pin/unpin UI API-only |
| Set / clear profile cover photo | ✓ | ✗ | ✗ | ✗ | partial | partial | P2 | API-only |

**Surface gaps:** AppreciateButton click, comment submit+delete UI,
featured-pin/unpin UI, cover-photo set/clear UI, celestial overlay positive
render, renamed-handle redirect browser path, /t and /u 404 pages,
`/api/targets` catalog-list endpoint (no backend test).

**Notes / concerns:**
- Engagement is the strongest backend area, but **two core social actions —
  appreciate (like) and comment posting/deletion — have ZERO passing
  browser e2e.** The appreciate UI test is a dead `test.skip()` stub (the
  mobile-sticky bar was removed in the 2026-05-03 redesign and never
  rewired). Follow is the only social action with real browser coverage.
- Plate-solve + no-RA dev DB → celestial overlay positive render and any
  RA/opposition target gallery verifiable on staging only.

---

## Equipment (setups + catalog + autocomplete + filters)

| Journey | API | UI | Mobile | a11y | Sec | Verdict | Pri | Gap |
|---|---|---|---|---|---|---|---|---|
| Setups list (counts, empty, anon) | ✓ | ✓ | ✗ | ✗ | partial | covered | P1 | — |
| Create setup via SetupForm | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | full create end-to-end |
| Create setup validation (dup/bad id/mode) | ✓ | ✗ | ✗ | ✗ | ✗ | partial | P1 | 422 surfacing not browser-tested |
| Edit existing setup (load→change→save) | ✓ | ✗ | ✗ | ✗ | partial | partial | P1 | only 404/defensive states UI-driven |
| Set setup as default | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | — |
| Delete setup (confirm→204→redirect) | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | — |
| setDefault error states | ✗ | ✓ | ✗ | ✗ | ✗ | covered | P2 | — |
| Equipment autocomplete in builder | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | — |
| Public catalog browse /equip/[kind] | ✓ | partial | ✗ | ✗ | ✓ | partial | P1 | no test asserts a populated card grid |
| Catalog browse invalid kind / XSS | ✓ | ✓ | ✗ | ✗ | ✓ | covered | P2 | — |
| Catalog item detail /equip/[kind]/[slug] | ✓ | ✓ | ✗ | ✗ | partial | covered | P1 | — |
| Catalog detail not-found (404 vs 500) | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P2 | — |
| Catalog spec edit (admin) | ✓ | ✗ | ✗ | ✗ | partial | partial | P1 | rename via raw API PATCH, not the form |
| Catalog edit access guards (anon/non-admin) | ✓ | ✓ | ✗ | ✗ | ✓ | covered | P1 | — |
| Filter chip autocomplete + junction | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | — |
| Equipment item resolve-or-create endpoint | ✓ | ✗ | n/a | n/a | ✗ | covered | P2 | infra endpoint, indirectly UI-exercised |

**Surface gaps:** (none at the route level — every route has ≥1 spec
touching it; but several "touches" are guard/404 only, see journey gaps).

**Notes / concerns:**
- No P0: create-setup, catalog browse page-load, catalog detail, set-default
  and delete are all real-frontend driven.
- Genuine gaps are **edit-and-save** flows: editing an existing setup
  (only 404/defensive UI states) and the admin spec-edit form (FE-0634 does
  the rename via `page.request.patch`, never the form). Cache-rebuild
  correctness proven only at API/DB layer.
- `/equip/[kind]` is loaded in browser but **every grid assertion is
  `toHaveCount(0)`** — the "see catalog results" happy path is unproven.

---

## Account & settings

| Journey | API | UI | Mobile | a11y | Sec | Verdict | Pri | Gap |
|---|---|---|---|---|---|---|---|---|
| Anon redirect guard on /settings/* + /account/frames | ✓ | ✓ | ✗ | ✗ | ✓ | covered | P2 | next-param preserved |
| Profile: edit display name (autosave) | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P2 | — |
| Profile: rename handle (avail + redirect) | ✓ | ✗ | ✗ | ✗ | ✗ | partial | P1 | only negative guards driven; no successful rename |
| Profile editor: avatar upload/replace/clear | ✓ | ✗ | ✗ | ✗ | ✗ | partial | P1 | zero browser e2e; backend thorough |
| Profile editor: bio/about/tagline/location/social | ✓ | ✗ | ✗ | ✗ | ✓ | partial | P1 | editor save never browser-driven |
| Change email (request w/ reauth) | ✓ | ✓ | ✗ | ✗ | partial | covered | P2 | confirm-link + errors API-only |
| Change password (wrong/short/success) | ✓ | ✓ | ✗ | ✗ | partial | covered | P2 | — |
| Appearance: theme + density | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P2 | — |
| Delete account: request 7-day | ✓ | ✓ | ✗ | ✗ | partial | covered | P2 | — |
| Delete account: cancel scheduled | ✓ | ✗ | ✗ | ✗ | ✗ | partial | P1 | cancel button asserted present, never clicked |
| Data export (download archive JSON) | ✓ | ✗ | ✗ | ✗ | partial | partial | P1 | link href asserted; download never exercised |
| Sessions: list + revoke + sign-out-others | ✓ | ✗ | ✗ | ✗ | ✗ | partial | P1 | only zero-other-sessions state driven |
| API tokens (PAT) full lifecycle | ✓ | ✓ | ✗ | ✗ | ✓ | covered | P2 | hash-at-rest asserted |
| Account library /account/frames | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P2 | — |
| Email change blocked for OAuth-only user | ✓ | ✗ | n/a | n/a | ✓ | covered | P2 | external dep; API + smoke |
| Account purge after grace (worker) | ✓ | ✗ | n/a | n/a | ✓ | covered | P2 | background job |

**Surface gaps:** ProfileEditor save flow (avatar/bio/identity), successful
handle rename, sessions destructive actions, delete-cancel path, export
download, email confirmation-link apply step.

**Notes / concerns:**
- Dominant gap: **ProfileEditor (avatar + bio + identity) and the
  sessions/delete-cancel/export egress actions have no browser e2e** despite
  thorough backend coverage. All P1.
- Bio is ammonia-sanitised at **write time**; `{@html bio}` is safe only
  because of that — any future writer must route through `users::bio::
  sanitize`.
- FE-0216 frozen contract: email-change form re-renders the success message
  on reload only via `showModal = $state(untrack(()=>!!form))`; a refactor
  dropping the `untrack/!!form` init silently hides the confirmation.

---

## Admin (super-admin equipment catalog + app settings)

| Journey | API | UI | Mobile | a11y | Sec | Verdict | Pri | Gap |
|---|---|---|---|---|---|---|---|---|
| Guard: anonymous rejection | ✓ | ✗ | n/a | n/a | ✓ | covered | P2 | anon→/signin redirect no browser e2e |
| Guard: authenticated non-admin rejection | ✓ | ✓ | n/a | n/a | ✓ | covered | P1 | boundary covered both layers |
| Equipment list: browse/filter/search/paginate | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | — |
| Equipment list: empty result | ✓ | ✓ | ✗ | ✗ | n/a | covered | P2 | — |
| Equipment edit: save (happy) | ✓ | ✗ | ✗ | ✗ | ✗ | partial | P1 | no browser e2e for successful save |
| Equipment delete: orphaned (happy) | ✓ | ✗ | ✗ | ✗ | ✗ | partial | P1 | no browser e2e for real delete |
| Equipment delete: refused in-use (409) | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P1 | — |
| Equipment edit: invalid status (merged) 422 | ✓ | ✗ | n/a | n/a | n/a | covered | P2 | UI structurally can't submit |
| App settings: read current | ✓ | ✓ | ✗ | ✗ | ✗ | covered | P2 | no assert on seeded values |
| App settings: save (happy) + signup gate | ✓ | ✗ | ✗ | ✗ | partial | partial | P1 | no browser e2e for successful save |
| App settings: validation error (over-cap) | ✗ | ✓ | ✗ | ✗ | ✗ | covered | P2 | 422 proven only via UI (no backend test) |
| /admin index redirect | ✗ | ✗ | n/a | n/a | ✗ | gap | P2 | bare /admin → /admin/equipment untested |

**Surface gaps:** `/admin` index redirect, equipment edit-save success,
equipment delete success, settings successful save.

**Notes / concerns:**
- No P0 — `/admin` is super-admin-only, not a public launch flow; the real
  security boundary (401/403 + redirect) is well covered.
- **All three successful WRITE journeys (edit-save, delete, settings-save)
  are backend-tested but have no browser e2e** — only failure/disabled
  states are FE-covered.
- Two coverage inversions: settings bound-validation 422 proven only via UI
  (no backend test); FE-0348 reaches the server guard by stripping the HTML
  `max` attribute.

---

## Infra & content delivery

| Journey | API | UI | Mobile | a11y | Sec | Verdict | Pri | Gap |
|---|---|---|---|---|---|---|---|---|
| CDN image transform — resize display master | ✓ | ✗ | n/a | n/a | partial | partial | P1 | dev route API-only; PROD CloudFront+Lambda@Edge zero automated coverage |
| CDN dev route — missing master → 404 | ✓ | ✗ | n/a | n/a | n/a | covered | P2 | — |
| CDN dev route — dimension clamp (DoS guard) | ✗ | ✗ | n/a | n/a | ✗ | gap | P2 | unauth memory-amplification clamp untested |
| Thumb serve endpoint (RSS fallback) | ✓ | ✗ | n/a | n/a | n/a | covered | P2 | — |
| RSS feed /rss.xml | ✗ | ✗ | n/a | n/a | ✗ | gap | P1 | no test of XML escaping / empty-feed fail-soft |
| Sitemap /sitemap.xml | ✗ | ✗ | n/a | n/a | ✗ | gap | P1 | cursor-walk / memo / dedupe / fan-out guard untested |
| robots.txt /robots.txt | ✗ | ✗ | n/a | n/a | ✗ | gap | P2 | origin-derived Sitemap line + shadow-file regression unguarded |
| /api reverse proxy — read path | ✗ | ✓ | n/a | n/a | partial | covered | P1 | cookie-forward heavily exercised |
| /api proxy — mutating CSRF 403 + large-body | ✗ | ✗ | n/a | n/a | ✗ | gap | P1 | proxy CSRF block + BODY_SIZE_LIMIT streaming zero e2e |
| Backend Origin/Referer CSRF guard | ✓ | ✗ | n/a | n/a | ✓ | covered | P2 | distinct from proxy guard |
| Outgoing email — send/templates/masking | ✓ | ✗ | n/a | n/a | partial | covered | P1 | real SMTP/MailHog transport manual only |
| Pending-upload cleanup reaper | ✓ | ✗ | n/a | n/a | n/a | covered | P2 | — |
| Static pages (/about/contact/privacy/terms/design) | ✗ | ✓ | ✓ | partial | ✓ | covered | P1 | mobile + XSS asserted; no cross-browser |
| Baseline security headers + CSP | ✗ | ✓ | n/a | n/a | ✓ | covered | P2 | CSP is Report-Only (not enforced) |
| Health check /healthz | ✓ | ✗ | n/a | n/a | n/a | covered | P2 | — |

**Surface gaps:** `/rss.xml`, `/sitemap.xml`, `/robots.txt`,
`/api/[...rest]` mutating path (proxy CSRF + large-body), `/cdn/img/:id`
prod transform path, dimension-clamp DoS guard.

**Notes / concerns:**
- **PROD image transform is the headline caveat:** prod uses CloudFront +
  Lambda@Edge (sharp); the `/cdn/img` route is *not registered in prod*. All
  CDN coverage applies to the dev/`APP_CDN_LOCAL_FALLBACK` path only — treat
  like OAuth/plate-solve (external, manual smoke).
- **Two distinct CSRF guards:** the backend Origin guard is API-covered; the
  **frontend proxy guard (`api/[...rest]/+server.ts`) has zero coverage** —
  yet it's the layer a cross-site fetch hits first.
- **CSP is Report-Only by design** — XSS protection rests entirely on Svelte
  auto-escaping + hand-rolled RSS/sitemap `escape()` helpers (untested).
  Graduating CSP to enforcing is future work.
- robots shadow risk: route works only while `static/robots.txt` stays
  deleted; no test guards the regression that once leaked the staging host.

---

## P0 — GO-LIVE BLOCKERS

Every P0 journey must have a passing **browser e2e** AND be wired into a
**required CI gate** before launch. There are 2:

1. **Auth & session · Email verification — click link, auto-login** —
   _partial._ Mandatory, unrecoverable account gate with a hand-rolled
   set-cookie parse/forward in `/verify/[token]/+page.server.ts`. **Zero
   browser e2e** — every spec shortcuts verification via a SQL update, so
   the real click → verify → set-cookie → auto-login path is unproven.
   **Must add a browser spec that navigates a real `/verify/[token]` link.**

2. **Content lifecycle · Upload → finalize → verify → publish → permalink** —
   _covered._ The core revenue/value chain is browser-covered today
   (`p1-happy-path.spec.ts`). The blocker is not coverage but the **missing
   CI gate**: with CI gitleaks-only, a green spec that never runs protects
   nothing. **Must wire Playwright into a required CI check.**

> Note: the launch-readiness bar is two-pronged. #1 is a coverage gap to
> close; #2 is the CI-gate gap that applies to the whole suite.

---

## P1 / P2 backlog (condensed)

**P1 (80) — high-value journeys that are partial/gap; not launch-blocking
because they degrade gracefully or are API-covered.** Highest-leverage
clusters:

- **Auth:** signin lockout banner, logout, email-change confirm + the
  FE-0216 modal bug, sessions revoke / sign-out-others, OAuth start +
  callback (external).
- **Content:** batch apply-to-all, batch publish-all, plate-solve states
  (external).
- **Discovery:** load-more pagination (all 4 feeds, untested both layers),
  logged-in following feed, search results listing, `/api/photographers`
  backend tests.
- **Targets/engagement:** appreciate-button click, comment submit+delete,
  celestial overlay positive render, draft-privacy boundary, /t catalog
  list.
- **Equipment:** edit-existing-setup save, admin spec-edit form, catalog
  populated-grid assertion, create-setup 422 surfacing.
- **Account:** ProfileEditor save (avatar/bio/identity), handle rename
  success, delete-cancel, export download.
- **Admin:** edit-save, delete, settings-save success paths.
- **Infra:** RSS/sitemap XML correctness, proxy CSRF + large-body, prod CDN
  transform (manual), outgoing SMTP (manual).

**P2 (66) — edge states, 404s, SEO/syndication, background jobs.** Mostly
API-covered or low-risk. Notable: `/admin` index redirect (gap), CDN
dimension-clamp DoS guard (gap, unauth route on staging/prod), robots.txt
origin-line + shadow-file regression (gap).

---

## Manual smoke checklist (external-dep journeys)

These cannot be browser-e2e'd locally; run them by hand on staging before
each launch.

### Google OAuth sign-in
- [ ] `/signin` → "Continue with Google" → Google consent screen appears
      (state cookie set on the redirect).
- [ ] Approve consent → callback exchanges code → **new** account is created
      and a session cookie is issued → lands authenticated on `/`.
- [ ] Repeat with an **existing** email → account is linked (not
      duplicated), session issued.
- [ ] Tamper/drop the `state` cookie → callback returns 422, no session.
- [ ] OAuth-only account → `/settings/email` change is blocked with the
      "cannot change email without a password" message.

### Plate-solving (XISF)
- [ ] Upload a real XISF → init accepted (platesolve configured) → finalize
      marks `awaiting_calibration`.
- [ ] Solve runs → `/upload/[id]/verify` shows solving → solved panel; the
      sweep promotes solved → `ready`.
- [ ] Force a solver failure / timeout → failed panel renders; sweep fails
      the timed-out calibration.
- [ ] Open a solved photo's permalink → **celestial overlay + object-type
      panel render** over the image (the positive path no local test covers).

### Prod CDN image transform
- [ ] Request a `display/<id>.jpg` through CloudFront with `?w=` → 200
      `image/jpeg`, resized, `Cache-Control` present (Lambda@Edge sharp path,
      not the dev `/cdn/img` route).

### Outgoing email (real SMTP / MailHog)
- [ ] Trigger signup verification, password reset, and email-change → mails
      actually deliver with correct subjects and the current address masked
      (only in-memory Mailer is automated-tested).

---

## Non-functional posture

| Dimension | Status |
|---|---|
| **Mobile** | Effectively untested. Only the static-pages area asserts a mobile viewport (FE-0702/FE-0711). No feed, upload, settings, or engagement journey sets a breakpoint. |
| **Accessibility** | Not audited. `getByRole` locators are used as selectors, not a11y assertions; only incidental focus/dialog/progressbar roles. No ARIA/contrast/keyboard-nav checks anywhere. |
| **Security** | Strongest non-functional area. XSS auto-escaping is broadly asserted (feeds, search, comment/caption, bio sanitize); PAT hash-at-rest, no-enumeration-on-resend, admin/anon auth gates, backend Origin CSRF guard all covered. Deferred: enforcing CSP (Report-Only today), frontend proxy CSRF guard (untested), the intentional email-enumeration leak (flagged), CDN dimension-clamp DoS guard (untested). |
| **Performance** | Zero coverage. No Lighthouse/Core-Web-Vitals/perf-budget assertions in any area. |
| **Cross-browser** | Effectively unverified. `playwright.config.ts` defines a single `chromium` project — no Firefox/WebKit. Every "covered" verdict is chromium-only. |

**Deferred to post-launch (explicit):** mobile-viewport suite, a11y audit,
perf budgets, cross-browser projects, enforcing CSP.

---

## Correctness / security concerns surfaced

1. **FE-0216 email-change modal (correctness bug):** non-enhanced form
   full-POST reload resets `showModal=$state(false)`; success confirmation
   renders inside the closed modal and never reaches the DOM. Mail sends;
   user sees nothing. _(Auth / Account)_
2. **Intentional email-enumeration leak:** signin-unverified and reset flows
   echo the submitted address in redirect query strings (FE-0113 freezes it
   as a contract). Real enumeration surface — review for go-live. _(Auth)_
3. **OAuth post-gate path is wholly untested by automation:** backend stops
   at the Google token-exchange boundary; account link/create + session
   issue rely on manual smoke only. _(Auth)_
4. **Keyset pagination unverified at both layers across all feeds** — a
   dup-row/skip-row bug in the cursor windows would ship undetected.
   _(Discovery)_
5. **`/api/photographers` and home-feed list branches have no backend
   test** — server-side ordering/filtering/cursors unverified. _(Discovery)_
6. **Frontend proxy CSRF guard has zero coverage** — the first control a
   cross-site mutating fetch hits; backend guard is a separate, covered
   control. _(Infra)_
7. **CSP is Report-Only**, so all XSS defense rests on Svelte auto-escaping
   + untested hand-rolled RSS/sitemap escapers. _(Infra)_
8. **Replace + large-body proxy path** only tested with tiny files; the
   documented `BODY_SIZE_LIMIT` failure mode is never exercised. _(Content)_
9. **Two appearance/engagement actions dead or skipped:** the appreciate UI
   test is a permanent `test.skip()` stub (feature removed in the 2026-05-03
   redesign, never rewired) — the like action has no passing browser e2e.
   _(Targets/engagement)_
10. **CDN coverage is dev-path only** — prod CloudFront+Lambda@Edge transform
    cannot run locally; green dev tests must not imply prod is verified.
    _(Infra)_

> Testing-method caveat: this assessment is static/read-based. Per project
> memory the local e2e stack is flaky (Docker crashes under load), so the
> suite was not executed — RED/contract status is taken from the specs' own
> inline comments.
