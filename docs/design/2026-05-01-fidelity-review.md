# Astrophoto — Visual Fidelity Review

**Date:** 2026-05-01
**Method:** Side-by-side screenshot comparison (1440 desktop, 720 signup, 390 mobile) between the React/CSS prototype at `docs/design/handoff/` and the SvelteKit production routes.
**Tooling:** Chrome DevTools MCP, headless Chrome.
**Screenshots:** `/tmp/astrophoto-shots/` (numbered `01..09` for SvelteKit, `p01..p07` for prototype).

## Verdict

**Desktop fidelity is high.** All seven desktop screens render the design language faithfully: warm-near-black backgrounds, sodium-amber accent, Source Serif 4 / Inter / JetBrains Mono trio, Reticle logomark, corner registration marks on hero photos, italic-emphasized words in display headlines, mono technical chrome.

**Mobile fidelity needs one round of follow-up work** (the photo detail page on a 390 px viewport falls back to a desktop-style layout instead of the prototype's mobile-specific shell).

A handful of small drift items (copy, dots, decorations) on signin/signup/404 can be corrected in a single small PR.

## Per-screen findings

### 1. Public gallery (`/`) — desktop 1440
**Files:** `01-svelte-gallery-1440.png` vs `p01-proto-gallery-1440.png`
**Verdict:** ✅ Pixel-faithful.
- Hero split editorial / featured photo: identical structure, same H1 with italic "the night sky", same stats line, same buttons.
- Filter chips and SORT/VIEW segmented control: identical.
- 3-column masonry with varied heights and target/integration captions per photo: identical.
- "Load page 2 of 974" pagination: identical.
- The deterministic `Photo` placeholder generates the same nebula-like gradients on both sides because both use the same `target → palette` hash function.

### 2. Photo detail (`/photo/[slug]`) — desktop 1440
**Files:** `02-svelte-photo-detail-1440.png` vs `p02-proto-detail-1440.png`
**Verdict:** ✅ Pixel-faithful with one approved enhancement.
- Two-column `1fr | 380px` layout: identical.
- Black image stage with 4-corner amber reticles, zoom controls bottom-left: identical.
- Right aside: published eyebrow, italic "NGC 7000" + regular "North America Nebula", uploader row with amber avatar + Marie Dubois + Follow, caption paragraph, action row with hearts/comments/share, ACQUISITION RECORD strip, EXIF table all 11 rows: identical.
- The "= 18.0 hours" exposure total renders in amber `#e8a43a` (verified via DOM inspection) ✓.
- **Enhancement:** SvelteKit shows `AppFooter` at the bottom; the prototype was capped at a 1100 px artboard so it didn't render one. This is an addition the production page needs.

### 3. User profile (`/u/[username]`) — desktop 1440
**Files:** `03-svelte-profile-1440.png` vs `p03-proto-profile-1440.png`
**Verdict:** ✅ Pixel-faithful with one approved enhancement.
- 120 px circle avatar with display-serif initial: identical.
- "Marie *Dubois*" 64 px display headline with italic surname: identical.
- 4-stat mono row, equipment strip, tab nav, 4-column square grid: identical.
- **Enhancement:** SvelteKit shows `AppFooter` at the bottom; prototype artboard was capped at 1500 px.

### 4. 404 (`/+error.svelte`) — desktop 1440
**Files:** `07-svelte-404-1440.png` vs `p06-proto-404-1440.png`
**Verdict:** ✅ Faithful with two minor drifts.
- Reticle 88 px in amber, eyebrow, italic-emphasized H1, paragraph, action row: identical.
- **Drift A:** the mono technical block (REQUESTED / COORDINATES) has a 1 px border on a slightly raised surface in SvelteKit; the prototype renders bare text. Minor enhancement, but a deviation.
- **Drift B:** prototype headline wraps as "We pointed the scope / at *nothing*."; SvelteKit wraps as "We pointed the / scope at *nothing*." Same H1 text, just narrower container caused different break. Cosmetic.

### 5. Sign in (`/signin`) — desktop 1440
**Files:** `05-svelte-signin-1440.png` vs `p04-proto-signin-1440.png`
**Verdict:** ✅ Faithful with three minor drifts.
- 50/50 split with photo column + form column, italic display quote bottom-left, mono coordinates bottom-right, "Welcome back / to *your archive*" headline, Google button, OR divider, EMAIL/PASSWORD inputs, primary "Sign in" button: identical.
- **Drift A:** SvelteKit eyebrow is "● SIGN IN" (with amber dot); prototype is "SIGN IN" (no dot). Other prototype eyebrows do use the dot, so this is consistent with the design language but inconsistent with this specific artboard.
- **Drift B:** prototype's photo column legend reads "● ρ OPHIUCHI · 5H45M · A. DIMOV" (3 fields); SvelteKit has "● ρ OPHIUCHI · 5H45M" (photographer credit dropped).
- **Drift C:** SvelteKit added a small "By signing in you agree to the Terms and Privacy Policy." legal line below the submit button; prototype had no such line.

### 6. Sign up (`/signup`) — column 720
**Files:** `06-svelte-signup-1440.png` (rendered at 1440 wide; column max 720) vs `p05-proto-signup-720.png`
**Verdict:** ⚠️ Faithful in structure, several copy/element drifts.
- Column layout, OPEN AN ACCOUNT eyebrow, italic-emphasized H1, OR WITH EMAIL divider, three inputs, primary submit, mono terms paragraph: all present and well-positioned.
- **Drift A:** SvelteKit prepends the Reticle Mark to the "Astrophoto" wordmark at the top; prototype shows the wordmark alone. Lockup is correct elsewhere, but the prototype intentionally simplifies the signup top-of-page to wordmark only.
- **Drift B:** SvelteKit eyebrow has the "● " amber dot prefix; prototype has none.
- **Drift C:** reassurance copy diverges. Prototype: *"Free, no ads, no rankings. Your photos with their full technical record, kept for as long as you want them kept."* SvelteKit: *"Free to use. We treat every frame as a technical record and as a photograph. You keep all your data; you can delete your account at any time."* Both are on-brand; not the prototype's words.
- **Drift D:** input placeholders changed. "How others will see you" → "Marie Dubois". "you@somewhere.com" → "you@domain.com". "At least 10 characters" → "At least 12 characters".
- **Drift E:** SvelteKit added a "Use at least 12 characters." mono hint below the password input; prototype had none.
- **Drift F:** SvelteKit added "Already have an account? Sign in →" link below the terms line; prototype had none.
- **Drift G:** terms copy diverges. Prototype: *"By continuing you agree to our terms and privacy policy. We don't ask for, and never sell, your data."* SvelteKit: *"By creating an account you agree to the Terms and Privacy Policy. We don't track you across the web."*

### 7. Photo detail mobile (`/photo/[slug]` at 390 wide) — ⚠️ NEEDS WORK
**Files:** `08-svelte-photo-detail-mobile.png` vs `p07-proto-detail-mobile.png`
**Verdict:** 🔴 Significant fidelity gap. The mobile breakpoint does not currently produce the prototype's mobile-specific layout.

The prototype's mobile photo detail is a fundamentally different shell, not just a narrower desktop:

| Element | Prototype mobile | SvelteKit mobile (current) |
|---|---|---|
| Header | Custom 3-icon bar: `← / Astrophoto / ⋯` | Full desktop AppHeader (logo + 4 nav links + 220 px search + Sign in + Create account) crammed into 390 px |
| Photo overlay | Plain photo, no chrome | Photo with corner reticles + bottom-left zoom controls (`100% / fit / + / −`) |
| Caption | Short summary: "18 h total integration, narrowband SHO, Bortle 4. Processed in PixInsight." | Full 4-line desktop caption |
| EXIF panel | 5 rows, font-size 11 | All 11 rows, desktop typography |
| Action bar | **Sticky bottom**: 3 flex cells `♡ 248 / 💬 12 / ↗ Share` over a top border | Inline ghost buttons in the body, no stickiness |

This is the only screen where the visual difference is large enough that a designer comparing side-by-side will flag it.

### 8. /design preview page (SvelteKit-only)
**File:** `04-svelte-design-1440.png` (no prototype counterpart — this is the engineering-internal preview page, not part of the user-facing product)
**Verdict:** ✅ Renders all tokens, type scale, components (buttons/inputs/chips/EXIF/logos/photos/corner marks). Matches the design system rules from `styles.css`.

## Recommended follow-up

In priority order. None of these blocks shipping the design system; all are tractable in a single half-day PR.

### P1 — Mobile photo detail (the only real gap)

Add a 768 px breakpoint to `/photo/[slug]` that:
1. Replaces `<AppHeader>` with a 48 px three-icon mobile header (`← back / wordmark / ⋯ menu`) when `width ≤ 768 px`. Either use a small `MobileHeader.svelte` or media-query-hide the existing AppHeader and render the mobile version conditionally.
2. Hides corner reticles and zoom controls on the photo (CSS `@media (max-width: 768px) { .photo-controls, .corner-mark { display: none } }`).
3. Truncates or replaces the caption with the short version ("18 h total integration, narrowband SHO, Bortle 4. Processed in PixInsight.") at `≤ 768 px`. Either store both `caption` and `captionMobile` in the data, or CSS-clamp the long caption to a few lines.
4. Adds a sticky bottom action bar (3 flex cells `♡ 248 / 💬 12 / ↗ Share`) at the bottom of the viewport when on mobile; hide the inline action row.
5. Reduces the EXIF table font from 13/14 to 11 px on mobile.

### P2 — Signup copy and decoration

In `frontend/src/routes/signup/+page.svelte`:
- Remove the `<MarkReticle>` from the top-of-page wordmark; render `<Wordmark>` alone.
- Remove the `● ` prefix from the "OPEN AN ACCOUNT" eyebrow (or strip it via the `t-eyebrow` invocation here).
- Replace the reassurance paragraph with the prototype's exact copy: *"Free, no ads, no rankings. Your photos with their full technical record, kept for as long as you want them kept."*
- Restore the prototype's input placeholders: "How others will see you" / "you@somewhere.com" / "At least 10 characters". Drop the `Use at least 12 characters.` mono hint.
- Drop the "Already have an account? Sign in →" link.
- Replace the terms paragraph with: *"By continuing you agree to our terms and privacy policy. We don't ask for, and never sell, your data."*

(Optional: keep the "Sign in →" link if you believe the prototype is stricter than necessary on that — that's a UX call, but for the **fidelity** review the prototype wins.)

### P3 — Signin polish

In `frontend/src/routes/signin/+page.svelte`:
- Remove the `● ` prefix from the "SIGN IN" eyebrow.
- Add the photographer attribution to the photo column legend: "● ρ OPHIUCHI · 5H45M · A. DIMOV".
- Drop the "By signing in you agree to..." legal line below the submit button (or keep it — it's good UX. The prototype omits it.).

### P4 — 404 mono block

In `frontend/src/routes/+error.svelte`:
- Remove the border from the REQUESTED/COORDINATES mono block. Render as bare mono text inside the centered hero, matching the prototype.
- Headline wrap is container-driven; if the visual difference is bothersome, set the H1's `max-width` so it breaks at "scope / at nothing." like the prototype.

## What's been verified working

- All 6 user-facing routes return HTTP 200 with full SSR-rendered content (no client hydration tricks).
- 404 returns proper HTTP 404 with the custom error page.
- Google Fonts (Source Serif 4, Inter, JetBrains Mono) load on every page.
- Reticle SVG favicon serves correctly.
- Form actions on signin/signup are wired to a `fail(501)` stub with a friendly message ("Sign-in is not wired to the backend yet").
- Sodium amber accent `#e8a43a` is computed on accent-class elements (verified via DOM `getComputedStyle`).
- Italic-emphasized words in display headlines render in Source Serif 4 italic.
- The synthetic `Photo` placeholder generates deterministic per-target gradients identically to the prototype's React `Photo` component.
- `just check` passes clean (cargo fmt, cargo clippy, svelte-check, prettier, eslint).

## Reproducing this review

```bash
# Frontend dev server
cd frontend && pnpm dev   # serves on :5173

# Prototype HTTP server (the React/CSS reference)
cd docs/design/handoff && python3 -m http.server 8765

# Open in two windows side-by-side, navigate matching pages:
http://localhost:5173/                              ↔  http://localhost:8765/single.html?screen=gallery
http://localhost:5173/photo/ngc-7000-...            ↔  http://localhost:8765/single.html?screen=detail
http://localhost:5173/u/marie-dubois                ↔  http://localhost:8765/single.html?screen=profile
http://localhost:5173/signin                        ↔  http://localhost:8765/single.html?screen=signin
http://localhost:5173/signup                        ↔  http://localhost:8765/single.html?screen=signup
http://localhost:5173/notarealroute                 ↔  http://localhost:8765/single.html?screen=404
```

The single-screen wrapper is `docs/design/handoff/single.html` — written for this review. Other valid screens are `tokens`, `home`, `upload`, `myphotos`, `settings`, `empty`, `detail-mobile`. Those screens (Phase 5 in the implementation plan) have not been ported to SvelteKit yet.
