# Astrophoto — Design Brief, Phase 8 and Beyond

**For:** Web designer (continuation of the May 1 brief)
**Audience of the product:** Amateur astrophotographers (unchanged from v1)
**Status:** v0.6.0-engagement is shipped. This brief covers what's next.
**Date:** 2026-05-02
**Version:** 2.0 — supplements `docs/design/2026-05-01-design-brief.md`

---

## Where we are today

Six phases shipped. The product currently supports:

- Public gallery, photo detail, user profile (all SSR, SEO-able)
- Email/password + Google OAuth signup/login/logout
- Photo upload pipeline (multipart → S3-compatible storage → EXIF + thumbnails in `spawn_blocking` → published as `ready`)
- ♥ appreciations, plain-text comments (1-level, photo owner moderates), asymmetric follows, following feed on the logged-in home
- Avatar dropdown menu (Profile / Upload / Sign out)
- `/u/:uuid` profiles with real follower counts, owner photos, follow toggle
- All design tokens, components, screens from the v1 prototype faithfully ported

The original design system from `docs/design/handoff/` covers most of
what we need. Several screens from the original prototype were ported
as MVP and now need a second pass; several screens from the original
prototype have **not been ported yet** (Settings, My photos, Empty state);
several features have **no design yet** (notifications, password reset,
2FA, collections, equipment library).

This brief asks for design work across three time horizons:
**Phase 8** (close the MVP), **Phase 9–10** (refinement), and **Vision**
(directional — do not block the layout).

The brand position, audience personas, tone, and visual language from
the May 1 brief are unchanged. **Read that brief first** if you haven't —
this document references it heavily and does not repeat its claims.

---

## Phase 8 — Close the MVP

The product needs roughly four more design passes to reach what an
amateur astrophotographer would call "useful day to day." These are
ordered by my best guess at user value, but happy to be re-ordered.

### 8.1 — Account settings (port + extend `ScreenSettings`)

**Existing prototype:** `docs/design/handoff/screens-2.jsx::ScreenSettings`.
The proto sketches a 2-column page (240px nav + content) with sections
for Identity, Equipment, Notifications, Email & Security, Appearance,
Sessions, and Delete. Port that, then extend with the actual
forms.

**Design work needed:**

1. **Email change flow** — modal or inline form? When submitted, the
   backend sends a verification email; the change applies only after
   the user clicks a link. Visualise the pending state ("change pending —
   verification sent to new@email").
2. **Password reset flow** — both "I'm logged in and want to change my
   password" (in settings) and "I forgot my password" (from the
   `/signin` page). The latter is a 3-step flow:
   - Enter email → "If an account exists, we'll send a link"
   - Click link in email → land on `/account/reset?token=...`
   - Pick a new password → log in automatically
3. **TOTP 2FA setup** — page or section showing a QR code + manual
   secret + 6-digit verification input. Backup codes screen. The
   "I lost my phone" recovery flow needs design too (use a backup code
   to sign in, then re-set up TOTP).
4. **Account deletion** — destructive action. Confirmation pattern:
   re-enter password + type the word "DELETE" + click. Show what's
   deleted (your photos, comments, appreciations, follows) and what's
   anonymized (other people's comments mentioning you stay; their
   appreciations of your photos disappear with the photos). 7-day
   grace period or hard delete? Decide with engineering — depends on
   what the database schema supports cheaply.
5. **Sessions panel** — list of active sessions (browser/device/IP/last
   used), with "Sign out this session" affordance. The proto sketches
   this; finalise the table layout and the empty state ("only this
   session").

**Decisions for the designer:**

- Sectioned page (single scroll with sticky nav) vs. tabbed router
  (`/account/settings/security`)? Proto did sectioned. I lean
  sectioned for MVP — fewer URLs, less router overhead.
- Should "Equipment" live in Settings (preset gear inventory) OR be
  derived from upload metadata? Equipment library is feature 9.x
  below; in Settings for Phase 8 we just want a simple list to manage
  saved presets — or we defer it entirely.

### 8.2 — My photos dashboard (port `ScreenMyPhotos`)

**Existing prototype:** `screens-2.jsx::ScreenMyPhotos`. Sketches a
table view: thumbnail · target · captured · integration · status
chip · ♡ count · ⋯ menu. Port this.

**Design work needed:**

1. **The status chips** — `published`, `processing`, `failed`, `draft`.
   Currently we only have `processing` → `ready` → `failed`; no draft
   concept. Adding `draft` is a Phase 8 decision (see 8.3 below).
2. **Bulk actions** — select multiple photos → delete? change visibility?
   Or skip bulk for MVP (delete one at a time via the row's ⋯ menu)?
   Proto suggests checkboxes; I'd argue for skip-for-MVP.
3. **Empty state** — new user with zero uploads. The proto's
   `ScreenEmpty` covers this for the home; My Photos needs its own
   empty: "Nothing here yet. Upload your first frame →".
4. **Sort + filter** — by status, target, date. The toolbar pattern is
   already established in the gallery filter bar; reuse it.
5. **Edit affordance** — clicking a row should go to either the photo
   detail (current behavior) or a dedicated edit screen. Decide:
   target/caption are edited inline (modal? expanded row?) or via a
   full edit page (`/photo/:id/edit`).

### 8.3 — Drafts + replace flow

The original brief listed two related Phase 2 features that we
deferred:

- **Mark as draft / publish later** (default is publish; drafts are
  private until released)
- **Replace a published image with a reprocessed version** (common
  workflow — re-process and re-publish without losing comments / likes)

**Design work needed:**

1. **Upload as draft** — checkbox on the upload form: "Save as draft —
   only visible to me". A drafted photo lives at the same URL but
   returns 404 to non-owners.
2. **Drafts in My photos** — the dashboard shows drafts with a `DRAFT`
   chip and a "Publish" button on the row.
3. **Replace flow** — on the photo detail page, an option in the ⋯
   menu (owner-only) to replace the file. Modal: drag-drop replacement
   image + warn that EXIF/dimensions may change + "Replace" button.
   Keep the same `id` (URL stable, comments + appreciations preserved).
4. **Edit-in-place caption / target** — on the photo detail, owner can
   click the caption to edit inline, or click target to retag. Pattern
   borrowed from Notion / GitHub: hover reveals a tiny edit pencil.

### 8.4 — Notifications

**No existing prototype.** This is a from-scratch design.

**Design work needed:**

1. **Header bell icon** — left of the avatar. Badge shows unread count
   (cap at 9+). Click → dropdown panel.
2. **Dropdown panel** — same surface as `AvatarMenu`, 360 px wide. Lists
   the last 10 notifications. Empty state: "Nothing new tonight."
   Each notification: avatar + body + relative time (`Mark · liked your
   photo · 2h ago`). Click → relevant page (the photo, the profile, the
   comment thread anchor).
3. **Notification types** to design:
   - `appreciation` — "Marie liked your *NGC 7000* photo."
   - `comment` — "Pascal commented on your *M31* photo: 'What gain...'"
   - `follow` — "Stéphane started following you."
   - `mention` (Phase 9+) — "@you in a comment by Pascal."
4. **Full notifications page** — `/notifications` with the same shape
   but paginated. "Mark all as read" button. Group by day
   (`Today`/`Yesterday`/`Earlier`).
5. **Email opt-in** — Settings has a toggle: "Email me when:
   ♥ someone appreciates / 💬 someone comments / + someone follows".
   Default OFF for new accounts (no surprise emails).
6. **Email template** — single-photo digest, plain text + minimal HTML.
   Same restrained brand voice. Subject lines like
   "Marie commented on your *NGC 7000* photo".

**Decisions for the designer:**

- Mention support in Phase 8 or defer? I lean defer — mentions need
  autocomplete + a parser, both heavy.
- Push notifications (browser web push) — out of scope. Email + in-app only.

### 8.5 — Polish residual from earlier phases

Several small things shipped as MVP and deserve a second visual pass:

- **Following feed eyebrow** — currently the hero overlay reads
  `FRAME OF THE WEEK / target` regardless of feed source. When the user
  is logged in and the feed is personalised, the eyebrow should signal
  it: e.g. `FROM THE PHOTOGRAPHERS YOU FOLLOW · 12`. Subtle but it
  earns trust.
- **FollowButton on the photo detail** — currently the photo detail
  shows "Follow" next to the uploader avatar, but it links to the
  uploader's profile (not a real follow action). Decide: is the photo
  detail's follow a real action (with the same FollowButton component
  as on the profile) OR a navigation "see this photographer's other
  work" button?
- **"Untitled" hero tag** — when an uploaded photo has no `target`
  field, the gallery hero shows "Untitled" as the FRAME OF THE WEEK
  caption. Either fall back to the filename, or hide the caption block
  entirely and just show the photo. Designer's call.
- **Mobile photo detail action bar** — currently "Appreciate / 0
  comments / Share" are inert placeholders on mobile (the Phase 7
  AppreciateButton was scoped to desktop only). Mobile needs a
  responsive AppreciateButton that fits the sticky bottom bar.

---

## Phase 9–10 — Refinement

After the close-the-MVP wave, the next round of design work refines
the experience around the workflows practitioners actually do.

### 9.1 — Target pages (`/target/:slug`)

Aggregated views: "All photos of M31, across all users." This is one
of AstroBin's strongest features and makes the product useful as a
reference catalog, not just a personal gallery.

**Design work:**
- Header: target name in display serif italic (`*NGC 7000* / North
  America Nebula`), pulled from a curated catalog (deep-sky objects,
  planets, lunar features, etc.).
- Stats row: "N photos · M h total integration · K photographers".
- Sky context: small mini-chart showing the target's RA/Dec on a sky
  map (Phase 10+ — for now, just the constellation name).
- Filter strip: by integration time, by camera, by date.
- Photo grid: same masonry pattern as the public gallery.

### 9.2 — Collections (manual albums)

User groups a set of their own photos under a name and (optional)
description. One photo can be in many collections.

**Design work:**
- Collections tab on the user profile (already in the existing tab nav
  per the design system; just inert).
- Collection detail page: hero (collection name + description + photo
  count + total integration), grid of photos, with an "Add photo" CTA
  for the owner.
- Create flow: from My photos, select photos → "Add to collection" →
  pick existing or create new.
- Delete flow with confirmation.

### 9.3 — Equipment library

Users save their gear (telescope, camera, mount, filters) with model
names, focal lengths, sensor pixel pitch, etc. Then on upload, they
pick from a dropdown instead of retyping.

**Design work:**
- Equipment page (`/account/equipment` OR a Settings section).
- "Add gear" form: type (scope/cam/mount/filter) + brand + model +
  notes.
- Equipment cards: visual chip with the gear icon + name.
- Upload form integration: dropdown on each EXIF field that lets the
  user pick a saved item.

### 9.4 — Statistics dashboard

Practitioner vanity feature. The design brief called this out as
"pure vanity but the audience loves it."

**Design work:**
- Stats page on the profile (`/u/:id/stats` or as a tab).
- Total integration time, broken down by:
  - Target (top 10 most-imaged objects)
  - Camera / scope (% of integration)
  - Year / month (calendar heatmap?)
- "Deepest single photo" callout (longest integration).
- Restrained design — text-and-numbers, very few charts. Mono
  aesthetic for the data, display serif for the headline numbers.

### 9.5 — Search + targets browse

Currently the gallery has a search bar (top-right of the AppHeader)
that does nothing. Wire it.

**Design work:**
- Click → expands into a search overlay (full-screen on mobile, modal
  on desktop).
- Recent searches, recent targets, suggested photographers.
- Results page: tabs for Photos / Targets / Photographers, each with
  the appropriate card layout.

### 9.6 — RSS / iCal feeds

The brief listed this as "many practitioners maintain blogs and want
the photo feed to flow into them."

**Design work:** small — just confirm the format strings:
- `/u/:id/rss.xml` — last 50 photos by that user.
- `/rss.xml` — public gallery (newest first).
- `/u/:id/ical.ics` — capture dates as calendar events (more niche).

No real UI to design beyond a small "RSS" link on the profile.

---

## Vision — directional, do not block the layout

These are 12+ months out. The designer should keep the current layouts
flexible enough that they don't preclude these features, but should
NOT design them yet.

- **Plate-solving + annotated star-chart overlay** — when a user
  uploads a deep-sky image, an offline pipeline computes the precise
  RA/Dec center, scale, and rotation. The photo detail page gains a
  "show annotations" toggle that overlays catalog labels (Messier
  numbers, NGC IDs, bright stars) on the image.
- **Sky-map browse** — interactive sky map; the user pans across the
  sky and sees photos taken of that patch. Niche but iconic.
- **FITS / RAW upload** — practitioners want to upload the FITS or
  RAW file as the canonical record while the site serves a JPEG to
  viewers. The upload form gains a "FITS source" optional field.
- **AI-generated draft captions** — based on EXIF + plate-solve, the
  system suggests a caption ("18 hours of NGC 7000 in narrowband from
  a Bortle 4 site") that the user edits.
- **Capture-log integration** — import session metadata from N.I.N.A.,
  SharpCap, or APT logs to auto-fill date / exposures / equipment.
- **Direct messages** — practitioners often want to ask another user
  "what was your gain on this?" Currently they comment publicly.
- **Internationalization** — English first; expect French, German,
  Spanish, Italian, Polish in 18 months. Keep all copy externalisable.
- **Comment threading 2nd level** — currently flat. May go to 1-level
  reply (Twitter-style) but never deeper. Designer should NOT design
  this until usage data shows demand.
- **Notifications: web push** — requires service worker + permission
  flow + design for the OS notification card.

---

## Decisions I'd like the designer to make

These are open questions that shape Phase 8 design directly. None are
load-bearing engineering decisions; they are style/UX trade-offs.

1. **Settings layout** — sectioned scroll (proto pattern) or tabbed
   router? My penchant: sectioned for MVP.
2. **Drafts as a first-class concept** in Phase 8, or defer to Phase
   10? My penchant: defer. Upload-and-publish is the only flow most
   users will use; drafts add a dropdown chooser that's confusing.
3. **Account deletion** — hard delete vs 7-day soft delete? Hard
   simpler for engineering; soft kinder for users who change their
   mind. The brief says GDPR-clean, which the user expects = hard
   delete after grace period.
4. **Notifications page IA** — full page (`/notifications`) or
   dropdown-only (no full page, just the bell + dropdown + click-to-
   navigate)? My penchant: dropdown-only for MVP; add a page when
   the dropdown becomes too crowded.
5. **Equipment library scope** — full Phase 9 feature with library
   page, or a thin "saved gear" dropdown on the upload form, populated
   by previously-typed values, with no dedicated UI? My penchant:
   thin version, no dedicated page, until users ask for one.
6. **Mention syntax** — `@username` (Twitter) vs `@display name` (Slack
   no-spaces, Slack-with-spaces, etc.)? My penchant: defer the whole
   feature.

---

## Deliverables expected

### Phase 8 (close the MVP) — required

In priority order:

1. **Settings page** at three breakpoints (1440 desktop, 768 tablet,
   390 mobile), covering all sections in 8.1.
2. **My photos dashboard** at desktop + mobile (the proto's table
   doesn't work as-is on mobile — need the design pass).
3. **Drafts + Replace flows** (modal or page; designer's call).
4. **Notifications** — bell icon, dropdown panel, full page (if you
   want to design one), email template (mockup, not HTML).
5. **Polish items from 8.5** — small targeted mockups for the eyebrow,
   the photo-detail follow button, the empty-target fallback, and the
   mobile AppreciateButton.

### Phase 9–10 (refinement) — desirable

6. Target page layout
7. Collection detail page + create-collection modal
8. Statistics dashboard layout

### Hand-off format

Same as v1: Figma file, components organised as a library, semantic
tokens (`bg/canvas`, `accent/default`, `fg/muted`). Annotations on
non-obvious interactions or motion. Build on the existing token set
defined in `docs/design/handoff/styles.css` — do not re-invent the
color palette or type scale.

If you want to add new tokens (e.g. a `--bg-warning-tint` for the
unsaved-changes-in-settings banner), prefix them clearly and document
their semantic role.

---

## What's deliberately out of scope for this brief

- Any change to the design system itself (tokens, type scale,
  component primitives). The shipped system is stable and consistent;
  we should accumulate evidence before mutating it.
- Anything from the Vision section.
- Marketing pages (about, terms, privacy, contact) beyond the existing
  footer links — these can stay as plain typeset documents on the same
  shell, no special design needed.
- Internationalization affordances in the layout (language picker,
  RTL support) — defer with the rest of i18n.
- Native mobile apps — web only.

---

## Reference list

- v1 brief: `docs/design/2026-05-01-design-brief.md`
- Design handoff bundle: `docs/design/handoff/`
- Engineering specs: `docs/superpowers/specs/2026-05-{01,02}-*-design.md`
- Implementation plans: `docs/superpowers/plans/2026-05-{01,02}-*.md`

---

*Questions, pushback, and counter-proposals welcome. The product is
real now — feel free to play with it locally (`just dev`) and submit
critiques against the live UI as much as against this brief.*
