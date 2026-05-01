# Astrophoto — Design Brief

**For:** Web designer
**Audience of the product:** Amateur astrophotographers
**Status of the product:** Pre-launch. Engineering bootstrap is done; design has not started.
**Date:** 2026-05-01
**Version:** 1.0

---

## 1. The product in one paragraph

Astrophoto is a web application for amateur astrophotographers to upload,
organize, and share their images of the night sky. It treats every
upload as a *technical artifact* — preserving and surfacing camera
settings, equipment, and astronomical context (target, sky coordinates,
date and location of capture) — and as an *aesthetic object* worth
presenting beautifully. The product sits between a photo-sharing site
and a logbook: it must feel like a serious tool to a serious hobbyist
**and** like a place where their best work looks worthy.

---

## 2. Audience

### Primary persona — "The Practitioner"

- 35–65, predominantly male today but the audience is growing more
  diverse and we should not visually exclude anyone.
- Has spent €1,000–€20,000 on equipment (camera, telescope, mount,
  filters, computer, software). They take this seriously.
- Captures 1–10 hours of exposure on a single target across multiple
  nights, then stacks and processes for hours more in dedicated
  desktop software (PixInsight, Siril, Photoshop).
- Highly technical. They notice and care when:
  - EXIF/FITS metadata is rendered wrong or missing.
  - A unit is wrong (degrees vs arcminutes, ISO vs gain).
  - A "compress for web" pipeline mangles their image.
- Spends time on **AstroBin** (the dominant incumbent), Cloudy Nights
  forums, and r/astrophotography. They've already learned that
  astrophoto sites tend to be either *technical and ugly* or *pretty
  and shallow*. Astrophoto must be both.

### Secondary persona — "The Curious Beginner"

- 20–45, mixed gender, just bought their first camera or smart
  telescope (Vespera, SeeStar, Stellina).
- Doesn't yet know what RA/Dec means, can identify the Moon and
  maybe Orion.
- Wants to share what they captured, get feedback, learn the
  vocabulary by osmosis. Easily intimidated by jargon walls.

### Tertiary persona — "The Lurker"

- General public, arrives via Google search ("M31 Andromeda
  galaxy photo") or social.
- Won't sign up unless something pulls them in.
- We must serve them: public galleries, beautiful permalinks,
  good SEO landing pages.

### Designing for all three at once

- **The Practitioner** wants density, technical truth, and respect.
- **The Beginner** wants simplicity and not feeling stupid.
- **The Lurker** wants the image, large and beautiful, with a single
  affordance to "see more by this person".

The design must default to *gorgeous and clear*; technical density
should be available on demand (a click, a scroll, a panel) but never
in the way of the image.

---

## 3. Brand & tone

### Positioning

> A serious, beautiful home for the work amateur astrophotographers
> spend their nights making.

We are **not**: Instagram for space. A meme aggregator. A telescope
e-commerce frontend. A NASA fan site.

We **are**: A respectful, premium, slightly nerdy place where a
70-hour integration of NGC 7000 looks as monumental as it actually is.

### Tone of voice

- **Respectful** of the craft and the time invested.
- **Quietly confident** — no hype, no exclamation marks, no
  "amazing!".
- **Plain over clever** in copy. "Upload" beats "Share your cosmos".
- **Specific** when specificity helps ("Captured 14 March 2026,
  3h12m total exposure") rather than vague ("Beautiful!").

### Mood / aesthetic direction (open to designer interpretation)

**Words to evoke:**
observatory dome at 3am · technical drawing · star atlas ·
old NASA mission patch · darkroom · a Leica catalogue ·
the inside of a high-end DSO planetarium app

**Anti-words:**
sci-fi · rockets · neon space gradient · cartoon planets ·
generic "cosmic" Web 2.0 · purple-pink-cyan synthwave ·
Comic Sans · dashboards with 47 widgets

The product should feel like it was built by someone who has been
out under a real sky.

---

## 4. Visual direction (recommendations, not mandates)

The designer is free to deviate; below are starting points based on
research into the audience.

### Color

The image is the hero. The chrome must recede.

- **Default theme: deep dark.** Astrophotographers preserve their
  night vision; they will be using the site at 1am with their eyes
  dark-adapted. A pure black or near-black background protects the
  perceived contrast of the photos.
- **Accent: warm and minimal.** Consider an amber, rust, or muted
  ember tone for actions and highlights — it harmonizes with red
  flashlights used at observation sites, and won't blow the user's
  dark adaptation. Avoid bright cyan or pure-white accents.
- **Light theme as a secondary mode.** Some users will browse on
  phones in daylight. A clean light theme should exist; it is *not*
  the default.
- **No gradients on the chrome.** The only gradients in the product
  are inside the user's photographs.

### Typography

- A **technical sans** for UI (e.g. Inter, IBM Plex Sans, Söhne,
  Aktiv Grotesk). Should read well at 13–14px on dark.
- A **monospace** for technical data (EXIF, equipment, coordinates):
  e.g. Berkeley Mono, JetBrains Mono, IBM Plex Mono. Monospace
  signals "this is a measurement, not prose".
- A **single accent face** is optional — a refined serif (e.g.
  Söhne Mono, GT America, or a clean transitional serif) could be
  used sparingly for the wordmark and the largest titles. Astrophoto
  is not a magazine; do not over-typographize.

### Imagery & iconography

- Photographs are the only ornament. The UI must let them breathe:
  generous margins around hero images; never crop a photo without
  user intent.
- Icons should be a thin, geometric line set (e.g. Lucide, Phosphor
  Light) — not filled, not playful.
- Avoid stock illustrations of planets, rockets, or cartoon
  astronauts. If empty states need imagery, use a thin star-chart or
  technical-diagram aesthetic.

### Layout & density

- **Two density modes** worth considering:
  - **"Show me the work"** — gallery cards large, one image
    dominates.
  - **"Show me the data"** — for the practitioner, a denser table
    or grid view that exposes equipment and integration time.
- The user should be able to switch. Default depends on context (see
  screen inventory).

### Motion

- Restrained. Fades and small position transitions only.
- Image-load should reveal a low-quality preview that resolves; this
  matches how the photo itself was captured (build-up over time).
- No parallax. No floating particles. No starfield backgrounds —
  again, the photographs *are* the starfield.

---

## 5. Feature inventory

Three columns: **MVP** (will be built first, must be designed first),
**Phase 2** (will follow, design should anticipate), and **Vision**
(direction the product is going; design layout should not preclude).

### A. Authentication & account

| Feature | Phase | What it does |
|---|---|---|
| Sign up with email + password | MVP | Create an account. Email is unique. Password rules light (length only). |
| Sign in with email + password | MVP | Standard credential auth. Session is server-side, lasts 30 days. |
| Sign in with Google (OAuth) | MVP | One-click via Google. Creates an account if none. |
| "Forgot password" via email link | Phase 2 | Reset link valid for 1 hour. |
| Two-factor authentication (TOTP app) | Phase 2 | For users who want it. |
| Account deletion | Phase 2 | Hard delete, GDPR-clean. Confirmation step. |
| Email change / display-name change | Phase 2 | Profile settings page. |

### B. Image upload & processing

| Feature | Phase | What it does |
|---|---|---|
| Drag-and-drop or click to upload | MVP | Single-image upload. JPEG/PNG/TIFF, ≤ 50 MB. |
| EXIF extraction | MVP | Camera, lens, ISO, exposure, focal length, capture timestamp pulled automatically and shown in a confirmation step. User can override any field. |
| Thumbnail generation | MVP | 400 px and 1200 px JPEGs generated server-side. Uploads complete fast; thumbnails resolve over a few seconds — UI must show this as a state. |
| Caption + target field | MVP | Free text caption (max ~500 chars) + a "target" string ("M31", "NGC 7000", "Moon, mare Imbrium"). |
| Mark as draft / publish later | Phase 2 | Default is publish. Drafts are private until released. |
| Multi-image upload (mosaic, RGB channels) | Phase 2 | Treat a set as one "post" with a primary image and several supporting. |
| Replace a published image with a reprocessed version | Phase 2 | Common workflow: re-process and re-publish without losing comments/likes. |
| RAW / FITS support (preview only, not stored) | Vision | Many users shoot in FITS and want to upload that file as the canonical record while serving a JPEG to viewers. |
| Plate-solving | Vision | Optional offline pipeline that determines the exact RA/Dec center, scale, and rotation of the image. Surfaces an annotated star-chart overlay. |
| AI-generated draft captions | Vision | Suggested caption text based on EXIF + plate-solve results, fully editable. |

### C. Browsing & discovery

| Feature | Phase | What it does |
|---|---|---|
| Public homepage / latest gallery | MVP | A grid of recent published photos from all users. Default sort: newest first. |
| Photo detail page | MVP | Single image, large; caption; uploader name; capture date; full EXIF table; target. Public URL (`/photo/<slug>`). |
| User profile page | MVP | Avatar (or initial), display name, all of that user's published photos, oldest or newest first toggle. |
| Search by free text | MVP | Searches captions and target field. Trigram-based, forgiving of typos. |
| Filter by target | Phase 2 | Click a target name (e.g. "M31") to see all photos of that target across all users. |
| Filter by camera / equipment | Phase 2 | Same idea for camera bodies, telescopes, focal lengths. |
| Sort by integration time, popularity, recency | Phase 2 | Practitioners care about integration time as a quality signal. |
| Star-chart browse (sky map) | Vision | An interactive sky map (powered by plate-solving data) where the user pans across the sky and sees real photos taken of that patch of sky. |
| Tags & curated topics | Vision | Editor-curated topic pages ("April lunar eclipse", "Comet 12P/Pons-Brooks"). |

### D. Engagement & social (light)

| Feature | Phase | What it does |
|---|---|---|
| "Appreciate" button (single counter, no public list) | Phase 2 | Lets viewers signal value without becoming a popularity contest. |
| Comments | Phase 2 | Plain-text, threaded one level deep, moderation tools for the photo owner. |
| Follow another user | Phase 2 | "Following" feed on logged-in homepage. |
| Notifications (in-app + opt-in email) | Phase 2 | New comment, new follower, weekly digest. |
| Direct messages | Vision | Practitioners often want to ask another user "what was your gain on this?". |
| Public RSS / iCal of a user's gallery | Vision | Many practitioners maintain blogs and want the photo feed to flow into them. |

### E. The user's own dashboard

| Feature | Phase | What it does |
|---|---|---|
| "My photos" listing (private view) | MVP | All your photos, draft + published, with management actions (edit metadata, delete). |
| Collections / albums | Phase 2 | Group photos manually ("Spring 2026 deep sky"). One photo can be in many collections. |
| Equipment library | Phase 2 | Save your common gear (telescope, camera, mount, filters) and tag uploads from a dropdown instead of retyping. |
| Capture log integration | Vision | Import session metadata from N.I.N.A., SharpCap, or APT logs to auto-fill date, exposures, and equipment. |
| Statistics dashboard | Vision | Total integration time, targets by hours, "deepest" image — pure vanity but the audience loves it. |

### F. Cross-cutting

| Feature | Phase | What it does |
|---|---|---|
| Responsive layout — mobile, tablet, desktop, ultra-wide | MVP | Practitioners often have 4K monitors and want the image to dominate. Mobile is for browsing, almost never uploading. |
| Accessibility (WCAG 2.1 AA) | MVP | Keyboard navigation, focus states, alt text on user images defaults to caption, contrast meets ratios in both themes. |
| Internationalization | Vision | English first; expect French, German, Spanish, Italian, Polish in 18 months. Keep all copy externalisable. |
| GDPR compliance | MVP | Cookie banner only if a non-essential cookie is used (we shouldn't need one for MVP — only the session cookie, which is essential). Privacy policy + terms link in footer. |

---

## 6. Key user journeys

### Journey 1 — The Practitioner uploads their best image of the year

1. Lands on logged-in home, sees their "follow" feed and a quiet
   "Upload" affordance.
2. Drags a 30 MB processed JPEG onto the page.
3. EXIF gets parsed; the form pre-fills camera, lens, exposure,
   capture date. They tweak the target to "NGC 7000 — North America
   Nebula".
4. Adds a 3-paragraph caption explaining their stack — narrowband,
   filters, integration time.
5. Publishes. Sees a clean confirmation with a copy-to-clipboard
   URL.
6. Returns the next morning to find some appreciations and a
   comment asking about their gain setting.

**Design concerns for this journey:**
- The EXIF confirmation step must feel correct and trustworthy. If
  it pre-fills wrong values, this user will not tolerate it.
- The caption editor should let them paragraph-break and ideally
  include a fixed-width block for raw acquisition data (some users
  paste tables of sub-exposures).

### Journey 2 — The Beginner shares their first M42

1. Lands on public homepage from Reddit. Sees a striking grid.
2. Clicks one photo, scrolls past the image, sees "captured with a
   Canon EOS RP, 60s × 30 stacks, ISO 1600". Realizes this is real
   technical info.
3. Clicks "Upload" → prompted to sign up. Uses Google.
4. Uploads their phone-camera-through-telescope shot of M42.
5. The form looks the same as the practitioner's, but most fields
   are optional; they fill in only target ("Orion nebula M42") and
   a one-sentence caption.
6. Publishes. Their photo appears in the public gallery alongside
   the practitioners'.

**Design concerns for this journey:**
- Optional fields must not feel like blanks they failed to fill.
- The gallery must not visually rank by integration time or
  technical sophistication — beginners need to see their work
  alongside experts without feeling buried.

### Journey 3 — The Lurker on a phone in bed

1. Searches "Andromeda galaxy" on Google, lands on a photo detail
   page on mobile.
2. Image loads with a low-resolution placeholder, resolves over
   ~1 second.
3. Pinches to zoom; UI gets out of the way (auto-hide chrome).
4. Scrolls past the image to read the caption.
5. Taps the uploader's avatar to see more of their work.
6. Closes the tab without signing up. (That's fine — see them again
   in 3 weeks.)

**Design concerns for this journey:**
- Mobile photo detail must let them zoom *into the image* (DSO
  details are the entire point), without the chrome interfering.
- "More from this user" must be one tap away.
- Sign-up must be invitational, not modal.

---

## 7. Screen inventory (to design)

Order is rough priority for designing first.

### Public

1. **Landing / public gallery** — desktop + mobile.
2. **Photo detail page** — desktop + mobile. Two states:
   *image loading* and *fully resolved*. Include the EXIF
   panel layout (technical data, expandable).
3. **User profile (public view)** — grid of their photos, sort
   toggle, basic about line.
4. **Search results** — same grid as gallery, with the query
   echoed and result count.
5. **404 / 403 / 500** — quiet, not cute.

### Authenticated

6. **Logged-in home / following feed** — same grid pattern as
   public, but a different default sort and the upload affordance
   prominent.
7. **Upload flow** — drop zone, EXIF confirmation step, caption +
   target, publish/draft. May be one screen or two.
8. **My photos** — private listing with edit/delete actions.
9. **Photo edit screen** — same shape as upload's metadata step,
   but pre-filled.
10. **Account settings** — email, display name, password, sign-out
    of all sessions, delete account.

### Auth

11. **Sign up** — email + password + display name; "or sign up with
    Google" button.
12. **Sign in** — email + password; "or sign in with Google".
13. **Forgot password / reset** (phase 2 but design now).

### Phase 2 to anticipate (do not design fully, but do not preclude)

14. **Collection detail page** (curated set of photos by a user).
15. **Target page** ("All photos of M31", aggregated across users).
16. **Notifications dropdown** (header).
17. **Comments section** of a photo detail page.

---

## 8. Content & data the user sees

So the designer can typeset realistic values, here is what real
content looks like:

### A typical photo's metadata

```
Caption:     North America Nebula in narrowband, 18h total integration
             over 4 nights from a Bortle 8 site. Hubble palette
             (SHO), processed in PixInsight.
Target:      NGC 7000 (North America Nebula)
Captured:    14–17 March 2026
Camera:      ZWO ASI2600MC Pro (cooled CMOS)
Telescope:   Takahashi FSQ-106EDX4 @ f/5
Mount:       10Micron GM1000 HPS
Filters:     Antlia 3 nm SHO narrowband set
Exposure:    180 × 360 s = 18.0 hours
Gain / ISO:  100
Sensor temp: −10 °C
RA / Dec:    20h 58m 47s / +44° 19′ 53″
Field:       1.7° × 1.1°
Pixel scale: 1.92 arcsec/px
```

### Realistic caption styles

- **Practitioner long-form:** 3–6 paragraphs, technical
  vocabulary, often with a fixed-width block of acquisition data.
  May include filter charts.
- **Beginner:** one sentence. "First time! Canon EOS RP through
  my new SkyWatcher 8" Dob, single 30 s shot."
- **Aesthetic-only:** zero or near-zero text. Photo speaks.

The design must accommodate all three without feeling broken.

### Display names

Range from real names ("Marie Dubois") to handles ("StarHunter42")
to long pseudonyms ("CometChaser_Astro_2024"). Truncate gracefully.

### Image aspect ratios

All over the map. Common cases:
- **3:2** — DSLR full-frame.
- **4:3** — astronomy CMOS sensors (most common).
- **1:1** — cropped portfolio choice.
- **Wide panoramas** (3:1 and wider) — Milky Way panos.
- **Tall portrait** (1:2 or taller) — comet tails, lunar libration animations.

The grid must handle all of these without ugly cropping or
distracting whitespace. Either Pinterest-style masonry, or fixed
aspect cards with thoughtful crop-with-focal-point — the designer
decides.

---

## 9. Constraints & non-goals

### Constraints

- **Performance:** the gallery is mostly images. Critical path must
  be lightweight (no heavy JS for first render). LCP target:
  ≤ 2.5 s on 4G.
- **Accessibility:** WCAG 2.1 AA at minimum, for both dark and
  light themes. Keyboard navigation must reach every action.
- **Print fidelity** is not a concern.
- **No infinite scroll without an obvious "page" affordance** —
  practitioners want to share permalinks to "my gallery, page 4".
  Either paginate or provide both.
- **No autoplay video.** Time-lapse animations may exist later as
  user content; they will be opt-in to play.

### Non-goals (do not design for these)

- Stories / ephemeral content.
- Public popularity ranking ("top 100 photos this week").
- Advertising slots.
- E-commerce / equipment marketplace.
- Live-stream of a user's telescope.

---

## 10. References & anti-references

### Look at

- **AstroBin** (astrobin.com) — the dominant incumbent. Study what
  they get right (depth of technical data, equipment library) and
  what they get wrong (1990s visual density, walls of forms).
- **Telescopius** — for the "target page" pattern and sky-map
  interaction.
- **NASA APOD** — for the calm presentation of a single image with
  long-form caption beneath.
- **Are.na** — for the quiet, restrained, content-first feel.
- **Cabinet of Natural Curiosities** websites — quiet typesetting
  of technical data.
- **Apple's photography pages** for the iPhone — for "the photo
  dominates a generous canvas" pattern.
- **Old Leica catalogues** — for the type-setting of technical
  specs as design.
- **Stripe.com** — only for the cleanliness, precision, and the
  "no-marketing-sweat" tone. NOT for the gradients.

### Avoid

- **SpaceX-style hero gradients** — too marketing, too aggressive.
- **Generic "cosmos" purple/cyan** — the cliché of the genre.
- **Glassmorphism** — wrong feel for archival content.
- **Heavy dashboard chrome** — this is a *gallery* primarily.
- **Default Bootstrap / shadcn looks** — if the result could ship
  on any SaaS, we've failed.

---

## 11. Deliverables expected from the designer

### Tier 1 — required for engineering to start UI work

1. **Visual language definition**
   - Two themes: dark (default) and light.
   - Type scale (5–6 sizes, weights, line-heights).
   - Color tokens (background levels, text, accent, error/warning,
     borders).
   - Spacing scale.
   - Component primitives: button (3 sizes, 4 variants), input,
     select, link, card, badge, modal, toast, tabs.
   - Icon set or icon library choice.
2. **High-fidelity mockups** for these screens, in dark and light
   theme, at desktop and mobile breakpoints:
   - Public gallery / landing.
   - Photo detail (with EXIF panel — both collapsed and expanded).
   - User profile.
   - Upload flow (drop zone + EXIF confirmation + caption step).
   - Sign in & sign up.
3. **One animated micro-interaction prototype** for the photo
   loading state (low-quality → resolved).

### Tier 2 — desirable

4. **Wordmark / logotype** for "Astrophoto". Restrained. May simply
   be the word set in the chosen accent face.
5. **Empty-state illustrations** in the chosen line style (e.g. an
   empty gallery showing a thin star-chart with a single labeled
   star).
6. **Favicon / app icon** in the same restrained register.
7. **Open Graph card template** for shared photo permalinks.

### Tier 3 — phase 2 design, sketched only

8. Layout sketches for: collection page, target page, comments
   section, notifications dropdown.

### Hand-off format

- **Figma** preferred. Components organized as a library,
  variables for color/spacing tokens.
- **Naming**: tokens in semantic form (`bg/canvas`, `bg/raised`,
  `fg/primary`, `fg/muted`, `accent/default`, `accent/hover`)
  rather than literal (`gray-900`, `amber-400`).
- **Annotations** for any non-obvious interaction or motion.

---

## 12. Open questions for the designer to push back on

The designer is welcome — encouraged — to push back on any of the
following. None of these are decided:

1. **Masonry vs. fixed-aspect grid** for the gallery: which serves
   the variety of aspect ratios best without making the page feel
   chaotic?
2. **Density toggle** ("show me the work" vs "show me the data") —
   one design with a switch, or two distinct grid pages?
3. **Where does technical data live** on the photo detail page:
   side panel (desktop) → drawer (mobile)? Below the image? An
   expandable strip?
4. **Wordmark** — is "Astrophoto" the right product name, or should
   we consider variants? (Engineering is willing to rename if the
   designer/brand work argues for it.)
5. **Accent color** — amber/ember per the recommendation, or
   something else that matches the night-vision constraint? Any
   strong opinion welcome.
6. **Empty states for new users** — what does an account with
   zero uploads look like, and how do we make that feel like a
   beginning rather than a void?

---

## 13. Stack note (for the designer's awareness)

The product is being built with **SvelteKit (Svelte 5)** on the
front end and **Rust (axum) + PostgreSQL** on the back end. This has
two practical implications for design:

- Components will be reimplemented from the Figma — the engineer
  is not pulling from MUI or shadcn. A small, focused component
  set is therefore better than a large one.
- The site is **server-side rendered**: SEO is real and the design
  must work at first paint without JavaScript hydration tricks.

The designer does not need to be a Svelte expert. Standard Figma
deliverables are perfect.

---

*End of brief. Questions, pushback, and counter-proposals welcome.*
