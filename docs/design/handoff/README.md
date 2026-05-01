# Handoff: Astrophoto

## Overview

Astrophoto is a web application for amateur astrophotographers to upload, organize, and share images of the night sky. It treats every upload as both a **technical artifact** (camera, telescope, mount, filters, integration time, RA/Dec coordinates, etc.) and an **aesthetic object** worth presenting beautifully. The product sits between a photo-sharing site and a logbook — serious tool, beautiful gallery.

The brand position: **a quiet, premium, slightly nerdy place where a 70-hour integration of NGC 7000 looks as monumental as it actually is.** Reference points: AstroBin (depth of data), NASA APOD (calm presentation of one image), Are.na (quiet content-first), old Leica catalogues (technical typesetting).

## About the Design Files

The files in this bundle are **design references created in HTML/React** — prototypes showing intended look and behavior, **not production code to copy directly**.

The target stack (per brief) is **SvelteKit (Svelte 5)** + **Rust (axum) + PostgreSQL**. Components should be reimplemented from the designs into Svelte components — no MUI/shadcn import. Server-side rendering is required (SEO matters, must work at first paint without JS hydration tricks).

The HTML prototype uses React + inline Babel only because that is the prototyping environment; **do not ship the React code**.

## Fidelity

**High-fidelity (hifi).** Pixel-perfect mockups with final colors, typography, spacing, and interactions. All design tokens are documented and final. The "photos" in the prototype are CSS-generated nebula gradients (since real Wikimedia hotlinking was blocked); in production, the photos will be user-uploaded JPEG/PNG/TIFF files served via a CDN — the gallery layout, aspect-ratio handling, and chrome are what to recreate.

## Brand & Identity

### Wordmark
**"Astrophoto"** set in **Source Serif 4**, weight 600, italic optional. Refined transitional serif — feels like a star-atlas frontispiece, not a tech logo.

### Logomark — Reticle (locked direction, "Logo No. 03")
A telescope-finder reticle: a circle with crosshairs extending past the ring, an inner dashed sub-ring, a center dot, and three small constellation dots inside the ring connected by hairlines. SVG, 64×64 viewBox.

```svg
<svg width="56" height="56" viewBox="0 0 64 64" fill="none" stroke="currentColor"
     stroke-width="1.4" stroke-linecap="square">
  <circle cx="32" cy="32" r="22" stroke-width="1.6"/>
  <circle cx="32" cy="32" r="14" stroke-dasharray="2.5 2.5" stroke-width="1" opacity="0.7"/>
  <line x1="32" y1="2"  x2="32" y2="18" stroke-width="1.6"/>
  <line x1="32" y1="46" x2="32" y2="62" stroke-width="1.6"/>
  <line x1="2"  y1="32" x2="18" y2="32" stroke-width="1.6"/>
  <line x1="46" y1="32" x2="62" y2="32" stroke-width="1.6"/>
  <circle cx="32" cy="32" r="2.4" fill="currentColor" stroke="none"/>
  <circle cx="24" cy="26" r="1.4" fill="currentColor" stroke="none"/>
  <circle cx="40" cy="36" r="1.7" fill="currentColor" stroke="none"/>
  <circle cx="36" cy="22" r="1.1" fill="currentColor" stroke="none"/>
  <line x1="24" y1="26" x2="36" y2="22" stroke-width="0.8"/>
  <line x1="36" y1="22" x2="40" y2="36" stroke-width="0.8"/>
</svg>
```

The mark is rendered in **accent (sodium amber, `#e8a43a`)** by default; can flip to `currentColor` in monochrome contexts. Logo lockup: mark + 12px gap + wordmark, vertically centered. See `logos.jsx` for 7 alternate marks (Ursa, Orion, Cassiopeia, Lyra, Atlas medallion, Vega star, monogram) — kept in the file for future variations but **only the Reticle is in production use**.

## Design Tokens

### Colors — Dark theme (default)

Astrophotographers work at 1am with dark-adapted eyes. Pure-black accents would blow that adaptation. Background is a **warm near-black** (slight ochre tint), accent is **sodium-amber** (red flashlight family, preserves night vision).

| Token | Hex | Role |
|---|---|---|
| `--bg-canvas` | `#0c0a08` | Page background |
| `--bg-base` | `#100d0a` | Default surface |
| `--bg-raised` | `#16120e` | Cards, panels |
| `--bg-elevated` | `#1d1812` | Hover, popovers |
| `--bg-overlay` | `rgba(12, 10, 8, 0.86)` | Modal scrim |
| `--border-subtle` | `#221d17` | Hairlines |
| `--border-default` | `#2c2620` | Inputs, dividers |
| `--border-strong` | `#3a322a` | Buttons |
| `--fg-primary` | `#f8f1e6` | Headlines, copy |
| `--fg-secondary` | `#d6cdba` | Body |
| `--fg-muted` | `#9c9384` | Meta, captions |
| `--fg-faint` | `#6a6358` | Disabled |
| `--accent` | `#e8a43a` | Sodium amber — DEFAULT |
| `--accent-hover` | `#f0b455` | Hover |
| `--accent-press` | `#c98920` | Active |
| `--accent-dim` | `#7a5a18` | Borders on accent surfaces |
| `--accent-ink` | `#0c0a08` | Text on accent |
| `--success` | `#6b8e4e` | |
| `--warning` | `#c98920` | |
| `--danger` | `#a8453a` | |
| `--info` | `#6b7d8e` | |

### Colors — Light theme (secondary, daylight mobile)

| Token | Hex |
|---|---|
| `--bg-canvas` | `#f5f0e6` |
| `--bg-base` | `#faf6ec` |
| `--bg-raised` | `#ffffff` |
| `--border-subtle` | `#ebe3d2` |
| `--border-default` | `#d9cfb9` |
| `--fg-primary` | `#1a1610` |
| `--fg-secondary` | `#3d362a` |
| `--fg-muted` | `#6e6657` |
| `--accent` | `#b06a0e` |

### Typography

- **Display** — `Source Serif 4` (Google Fonts, optical-size axis 8..60). Weights 400/500/600/700, italic available. Used for headlines, photo target names, chapter titles. Default weight **600** for confidence on dark.
- **UI** — `Inter` (Google Fonts). Weights 400/500/600/700. Used for body, navigation, buttons, form labels.
- **Mono** — `JetBrains Mono` (Google Fonts). Weights 400/500/600. Used for **all measurements** — EXIF data, RA/Dec, exposures, eyebrow labels, nav links, meta lines. Monospace = "this is a measurement, not prose."

### Type scale

| Name | Size | Use |
|---|---|---|
| Display 88 | 88px | Hero (sparingly) |
| Display 64 | 64px | Page H1 |
| Display 48 | 48px | Section H1 |
| Display 32 | 32px | H2, photo title |
| Body 16 | 16px | Body, secondary |
| UI 14 | 14px | Default body, forms |
| UI 13 | 13px | Dense lists |
| Mono 13 | 13px | EXIF values |
| Label 11 | 11px, 0.16em tracking, uppercase | Labels, eyebrows |

Headlines mix italics on the *emphasized* word (e.g. "A quiet archive of *the night sky*") — `<em>` inside the H1.

### Spacing

4 / 8 / 12 / 16 / 20 / 24 / 32 / 40 / 48 / 64 / 80 — `--s-1` through `--s-20`. 8px base grid.

### Radii

Very minimal: `--r-sm: 2px` (default), `--r-md: 4px` (cards), `--r-lg: 8px` (rare). **No pill shapes except avatars.** No glassmorphism. Sharp corners signal "instrument, not toy."

### Shadows

`--shadow-sm: 0 1px 0 rgba(0,0,0,.4)` · `--shadow-md: 0 8px 24px rgba(0,0,0,.5), 0 1px 0 rgba(255,220,160,.03) inset` · `--shadow-lg: 0 24px 64px rgba(0,0,0,.7), 0 1px 0 rgba(255,220,160,.04) inset`

### Motion

- Easing — `cubic-bezier(.2,.7,.3,1)` (out) and `cubic-bezier(.6,0,.2,1)` (in-out)
- Transitions — 150ms for state changes, 600ms for image scale on hover
- **No parallax. No floating particles. No starfield backgrounds.** The photos *are* the starfield.
- Image-load reveal: low-quality preview → resolved (matches how astrophotos are themselves built up)

## Components

### Button

```css
.btn {
  display: inline-flex; align-items: center; justify-content: center;
  gap: 8px; height: 36px; padding: 0 16px;
  border-radius: 2px;
  font-family: var(--font-ui);
  font-size: 12px; font-weight: 600; letter-spacing: 0.01em;
  border: 1px solid transparent;
  transition: all .15s var(--ease-out);
}
```
Sizes: `.btn-sm` (28px tall, 12px pad, 11px font), default (36px), `.btn-lg` (44px tall, 24px pad, 14px font).

Variants:
- **Primary** — `bg: --accent`, `color: --accent-ink`. Hover: `--accent-hover`.
- **Secondary** — transparent, `border: --border-strong`, `color: --fg-primary`. Hover: border + text → `--accent`.
- **Ghost** — transparent, no border, `color: --fg-secondary`. Hover: `bg: --bg-elevated`, `color: --fg-primary`.
- **Danger** — transparent, `color: --danger`. Hover: border → `--danger`.

### Input / Select / Textarea

36px tall (auto for textarea, min-height 96px), `bg: --bg-base`, `border: 1px solid --border-default`, radius 2px, 14px sans. Focus: border → `--accent`, `box-shadow: 0 0 0 3px rgba(232,164,58,.12)`. Mono variant `.input-mono` switches to JetBrains Mono for technical fields (RA/Dec, gain, etc).

### Chip / Badge

```css
.chip {
  display: inline-flex; align-items: center; gap: 4px;
  padding: 3px 8px; border-radius: 2px;
  border: 1px solid var(--border-default);
  background: var(--bg-base);
  font-family: var(--font-mono); font-size: 11px;
  letter-spacing: 0.04em; color: var(--fg-secondary);
}
.chip-accent { border-color: var(--accent-dim); color: var(--accent); background: rgba(232,164,58,.06); }
```

### Photo card

A wrapping div, `position: relative`, `overflow: hidden`, no rounded corners on hero photos. Image `object-fit: cover`, `transition: transform .6s` — scales 1.015 on hover. **Optional overlay**: linear gradient from black 78% to transparent at 50%, holds caption text; `opacity 0 → 1` on hover.

### EXIF table

Class `.exif`. Two columns, 38% / 62%. Column 1: 11px mono uppercase muted (label). Column 2: 13px mono primary (value, multi-line allowed for unit clarification e.g. "180 × 360 s\n= 18.0 hours"). Rows separated by `1px dashed --border-subtle`.

### Header (`.app-header`)

64px tall, full-width, bottom border `--border-subtle`. Three-column flex: logo (mark + wordmark) | center nav | right (search + auth actions). Search field 220px wide, mono, with ⌘K hint. Nav links use `.nav-link` — mono 11px uppercase 0.14em tracking, 1px amber underline 2px below baseline when active.

### Corner registration marks

Decorative element — 14×14 px bracket in corners of hero images. `border-top + border-right` (or matching pair) in `--accent`, signals "calibrated frame." Used selectively on hero / featured photo.

## Screens

All desktop screens are **1440px wide**. Mobile is **390px wide**. SSR-friendly — no JS required for first paint.

### Public

#### 1. Public gallery / landing
**Purpose:** First impression for the Lurker; gateway to sign up for The Beginner.
**Layout:** Header → Hero (split 1fr 1fr: editorial copy left, full-bleed featured photo right with corner reticles + caption tag) → Filter/sort bar (chips for category, sort dropdown, view toggle) → **Masonry grid** (CSS `column-count: 3, column-gap: 20px`, photos at native ratio with column heights varying 280–540px) → Pagination footer ("Load page 2 of 974" — explicit page number per brief; no infinite scroll without pagination affordance).

#### 2. Photo detail (desktop)
**Purpose:** The single photo, large; full technical record on demand.
**Layout:** Header → 2-column grid (1fr | 380px sidebar). Left = pure black canvas, photo centered with 48px padding and corner reticles, zoom controls bottom-left (100%/fit/+/−). Right sidebar = title (`<em>NGC 7000</em><br/>North America Nebula` — 32px serif italic on first line, regular below) → published date eyebrow → uploader avatar + follow button → caption paragraph → action row (♡ N appreciations / N comments / Share) → **EXIF table** (collapsible, default expanded). EXIF includes: Target, Captured, Camera, Telescope, Mount, Filters, Exposure (mono total in accent), Gain, RA/Dec (special chars `ʰ ᵐ ˢ ′ ″`), Field, Pixel scale.

#### 3. Photo detail (mobile, 390px)
**Layout:** Top bar 48px (back ← / wordmark center / ⋯ menu) → photo 320px tall → scrollable body (title, uploader, caption, EXIF table at fontSize 11) → sticky 3-button bottom bar (♡ count, comment count, share). EXIF stays visible — beginners are encouraged to scroll past the photo and learn vocabulary by osmosis. Pinch-to-zoom on the image hides the chrome.

#### 4. User profile
**Layout:** Header → Hero (avatar 120px circle accent-bg + serif initial, name "Marie *Dubois*" 64px display italic on surname, about line, stats in 4 cells: frames / integration / followers / collections) → Equipment strip (mono row: SCOPE / CAM / MOUNT / FILTERS) → Tabs (Frames · 42, Collections · 8, Equipment, About) → 4-column square grid of frames.

### Auth

#### 5. Sign in
**Layout:** 2-column 50/50 split. Left = full-bleed photo (ρ Ophiuchi placeholder) with dark gradient overlay; logo top-left, italic display quote bottom-left, RA/Dec mono bottom-right. Right = centered 380px form column: eyebrow, "Welcome back / to *your archive*", "New here? Open an account →" link, "Continue with Google" secondary button, divider "OR", email + password inputs, primary "Sign in" button.

#### 6. Sign up
**Layout:** Single 720px column, 64px padding. Wordmark top, eyebrow, headline "A serious home for / *the work you make*", reassurance paragraph, Google button, divider "OR WITH EMAIL", display name + email + password inputs, primary "Create my account" button, terms paragraph in mono meta style.

### Authenticated

#### 7. Logged-in home (following feed)
**Layout:** Header (with auth chrome — Upload button + avatar) → Greeting hero ("Good evening, *Marie*" 56px display italic + "12 new frames from the people you follow · clear skies tonight in Provence") + big primary "+ Upload a frame" button → Tabs (Following · 12 / All public / Targets I watch) → **Asymmetric grid**: hero photo (2/3 width) + 2-up stack (1/3 width) → Recent activity log (mono table: who / action / target / time, dashed dividers) → 4-column "more from people you follow" grid.

#### 8. Upload flow
**Purpose:** EXIF confirmation step is the sacred moment for The Practitioner. **Must feel correct or trust is lost.**
**Layout:** Header → Page title with stepper (3 steps: 01 UPLOAD ✓ done / 02 VERIFY DATA active / 03 CAPTION & PUBLISH). Top borders on each step in `--accent` when done/active, `--border-default` otherwise → Body 2-column (560px image preview | rest is form): left has the photo with a "● PROCESSING THUMBNAILS 72%" mono progress chip overlay; right has 2-column grid of editable fields (Target, Captured, Sessions, Camera, Telescope, Focal length, Aperture, Mount, Filters, Exposure, Gain, Sensor temp, RA/Dec). Each field has a mono label + tiny meta tag ("from EXIF" in accent if auto-detected, "you fill" muted otherwise). All fields use `.input-mono`. Optional plate-solve callout box → bottom action row (Save as draft / Replace file / Continue to caption →).

#### 9. My photos dashboard
**Layout:** Header → Title row (h1 left, 4 stats right: published / drafts / total integration / appreciations) → Filter row (status chips + sort/filter dropdowns) → Table with columns: thumbnail (60×60) / Target (display italic) / Captured (date) / Integration / Status (chip) / ♡ count / ⋯ menu. Rows separated by dashed bottom border, 12px vertical padding.

#### 10. Account settings
**Layout:** Header → Page title → 2-column (240px nav | content). Nav = vertical mono list with active state (left amber border + amber-tinted bg). Sections: Identity, Equipment, Notifications, Email & Security, Appearance, Sessions, Delete (in --danger). Each section uses `<Section title desc>` (display italic h2 + muted desc) and `<Row label>` (140px mono label + 1fr field). Theme toggle and density toggle as chip groups.

### Edge

#### 11. 404
**Layout:** Header → centered hero on `.bg-grid` (faint amber 1px grid 48px tile). Big reticle mark (88px in --accent), eyebrow "● 404 · NO LIGHT FROM THIS DIRECTION", display headline "We pointed the scope at *nothing*.", paragraph, mono technical block ("REQUESTED · /photo/...\nCOORDINATES · UNRESOLVED"), action row.

#### 12. Empty state (new user)
**Layout:** Same shell. Big Atlas medallion (104px in --fg-faint, *not* accent — it's quiet), display headline "*An empty plate*, / waiting for first light.", reassurance paragraph, primary "Upload your first frame" CTA, footer link to short guide. **Tone matters most here** per brief — this is The Beginner's first authenticated view. Never feel like a void.

## Imagery & gallery layout policy

Aspect ratios are wildly varied (3:2, 4:3, 1:1, 3:1 panos, 1:2 portrait). **Decision: masonry** (CSS columns) for the public gallery and following feed. **Fixed 1:1 squares** for profile and dashboard listings. **Always preserve user crop** — never auto-crop a photo without explicit intent.

In production, photos resolve in two stages: a 400px low-quality JPEG (blurred, ~5KB) loads first, then the 1200px resolved version replaces it with a 600ms cross-fade. This mirrors how the photos themselves were captured (build-up over time).

## Interactions & behavior

- **Image hover** — scale to 1.015, 600ms ease-out
- **Buttons** — color/border transitions 150ms
- **Inputs** — focus ring (3px 12%-opacity amber)
- **Photo card overlay** — caption fades in opacity 0→1 on hover
- **EXIF panel** — chevron rotates, content height animates (or use `<details>` element for SSR-safety)
- **No autoplay video** — even when video content is added (Phase 2+), it's opt-in to play

## Accessibility (WCAG 2.1 AA)

- Keyboard navigation must reach every action
- Focus rings: 3px amber 12% opacity outside the element, never `outline: none` without replacement
- Alt text on user photos defaults to caption + target string
- Contrast ratios verified for both themes — `--fg-secondary` on `--bg-canvas` is the floor
- Skip-link to main on every page

## SSR / SEO

- Photo detail pages must render full content (image, caption, EXIF, uploader) at first paint without JS — they are the SEO surface
- Public gallery, profile, target pages: ditto
- Theme toggle reads from cookie set by SSR — no flash of wrong theme
- `<meta property="og:image">` uses the 1200px JPEG; Open Graph card template renders photo + wordmark + uploader name

## State management (Svelte/SvelteKit notes)

- Auth state in a SvelteKit hook + cookie-based session (per brief: server-side session, 30 days)
- Theme preference: `theme` cookie, applied via `<html data-theme="dark|light">`
- Density toggle: `density` cookie, two values (`work`, `data`) — controls profile/gallery card variant
- Photo upload uses a multi-step form; EXIF parsing happens server-side on first POST, then user reviews/edits before final publish

## Files in this bundle

- **`Astrophoto Design.html`** — entry point, opens design canvas with all artboards
- **`styles.css`** — full design tokens + base + component CSS. **This is the source of truth** for all token values; lift it directly into the Svelte project's global stylesheet.
- **`logos.jsx`** — 8 logo direction explorations + the locked Reticle mark (`MarkReticle`). Lift the SVG of MarkReticle for the production logo.
- **`shared.jsx`** — `AppHeader`, `AppFooter`, `Photo` placeholder generator
- **`screens-tokens.jsx`** — design-system overview artboard (typography + color + components)
- **`screens-1.jsx`** — Gallery, Photo detail (desktop + mobile), Profile
- **`screens-2.jsx`** — Logged-in home, Upload flow, My photos, Sign in/up, Settings, 404, Empty state
- **`design-canvas.jsx`** — pan/zoom canvas wrapper (prototyping only, not for production)

## Out of scope / non-goals

- Stories / ephemeral content
- Public popularity ranking (top 100, etc.)
- Advertising, e-commerce
- Live telescope streams
- Print fidelity

## Phase 2 to anticipate (do not preclude)

- Comments (threaded one level), collections, target pages, follow/notifications dropdown, equipment library, drafts, replace-image. Layout headers and detail pages are designed with the side rails / bottom rails ready to host these.

## Vision (further out)

- Plate-solving + annotated star-chart overlay on photos
- Sky-map browse
- FITS/RAW upload
- AI draft captions
- Capture-log integration (N.I.N.A., SharpCap, APT)

---

*If anything in this README is unclear, the source HTML in `Astrophoto Design.html` (with `styles.css`) is the authoritative reference. Open it, pan around, and pick the artboard.*
