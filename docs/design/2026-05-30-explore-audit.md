# /explore Design & Engineering Audit — astrophoto.pics

> **Date:** 2026-05-30 · **Surface:** `/explore` (Phase-3 cross-author Discovery)
> **Method:** code-vs-design conformance (6 readers) + external best-practice research
> (4 angles) + live prod/staging probe, synthesized into this report; key claims
> cross-verified by hand against the source and against first-hand
> chrome-devtools measurements (Appendix A).
> **Design baseline:** `docs/design/handoff-showcase/README.md` (Phase-3 Discovery
> governs `/explore`) + `docs/design/handoff/README.md` (tokens/base components) +
> `docs/superpowers/specs/2026-05-03-photographer-showcase-design.md` (spec is
> canonical for behavior/schema/URLs).

**Scope:** Phase-3 cross-author Discovery surface (`/explore`). Justified-rows layout is *intended* design, not a defect. Prod has ~1 published photo (live perf numbers not representative; staging `?since=all` = 7 photos across 3 authors is the populated comparison).

---

## Status (2026-05-30)

Implemented on branch `fix/explore-audit`:

- SSR-safe CSS-flex justified grid — real tiles/links/`<img>` now in the
  server-rendered HTML (closes the §3 SSR-empty-grid P0).
- Tiny-variant LQIP behind each tile.
- First-row `fetchpriority="high"` (LCP image no longer lazy).
- Eyebrow frame count now uses the site-wide total (not the first-page length).
- Canonical + OG/Twitter meta on `/explore`.
- App-wide skip-link.
- Lightbox focus-trap + focus restore.
- `prefers-reduced-motion` honored.
- Tile `:focus-visible` outline.
- Dynamic new-moon line (no longer a hardcoded date).
- Google Fonts de-blocked (no longer render-blocking).
- HTML security headers.
- Sitemap cap raised.
- Composite index migration for most-appreciated+category (**EXPLAIN-unvalidated**
  — confidence medium, see §4 / §8).

**Image format: investigated → JPEG retained** (no code change; see Appendix B).

Deferred (need schema/spec work before building): trending sort with time-decay,
faceted catalog search, collections/boards, "most discussed" sort, equipment-on-tile.

---

## 1. Executive summary

- **Overall verdict: structurally faithful, but one architectural gap dominates.** The grid correctly implements Flickr justified-layout at the design's parameters, the filter rail/header match the showcase mock, and the design token layer is byte-identical to the handoff. The dev-only cursor footer was correctly stripped. But the surface ships **zero photo tiles in server-rendered HTML** — that SSR gap is the one issue that dominates. (The image format was a candidate P0 but **dissolved on measurement**: JPEG is the smallest format at every width for this noisy astro content — see below.)
- **P0 — SSR renders ZERO tiles (confirmed live).** `CrossAuthorGrid` computes layout from a client-measured `containerWidth` that is `0` during SSR, so `layout.boxes` is `[]` and the `{#if layout.boxes[i]}` guard emits no `<a>`, no `<img>`, no caption — just `<div class="grid" style="height:0px">`. Curl proves it: prod and a *populated* 7-photo staging feed both return `/u/`=0, `/p/`=0, `<img`=0. Per-photo links are **not** in SSR HTML. SEO/LCP/no-JS regression.
- **Investigated → KEEP JPEG (was a candidate P0; dissolved on measurement).** Every explore image is JPEG (the frontend emits no `fm`; grep = 0), and the CloudFront cache policy strips `Accept` so the edge never negotiates — but for high-noise astrophotography that is the *right* output. Measured against a real published frame at three widths, **JPEG is the smallest format at every width**; WebP is consistently larger (e.g. 34.0 KB JPEG vs 39.9 KB WebP at w=400), and AVIF is both larger where it works (w=400) and **broken at the edge for w≥800** (returns a 1 KB `text/html` error page). Conclusion: **do not switch formats and do not add `avif` to the `Transform` type.** See Appendix B for the full grid. The AVIF edge error is tracked separately as a LOW-priority infra finding (§6 / §8 — the Lambda@Edge should fall back to JPEG, not return an error page).
- **P1 — eyebrow "PUBLISHED FRAMES" count is the first-page length (≤24), not the site total.** Latent today (1 photo) but will read "24 PUBLISHED FRAMES" on a site with thousands. The correct value already exists at `GET /api/site/stats`.
- **P1 — accessibility cluster, patterns already exist in-repo but unused:** no skip-link anywhere; Lightbox has no focus trap / move-in / restore (whereas `Modal.svelte` does all three); no `prefers-reduced-motion`; no canonical/OG/Twitter tags on `/explore` (whereas `PhotoDetailFull.svelte` has the full set).
- **Live measurement (Appendix A):** LCP 480 ms / CLS 0.04 on the near-empty prod page, **LCP element is the H1 text — no photo exists to be LCP** (corroborates the SSR gap); Lighthouse A11y/SEO/Best-Practices = 100 **with JS** (does *not* clear the no-JS gap); lone Lighthouse failure is a missing `/llms.txt`.
- **Resolved (2026-05-30):** the two handoffs disagreed on chip radius — original handoff says sharp 2px, showcase said ~999px pill. Reconciled in favour of **sharp 2px** (the shipped behavior + the token-authoritative original handoff); the showcase README's "pill" line was struck. No code change needed.
- **Two legacy detail components in `discovery/` carry a stale cool-gray + blue-accent template palette** (`var(--bg-faint, #0a0a0a)` renders live because `--bg-faint` is undefined). The explore-grid components proper are token-clean.

---

## 2. Design conformance matrix

Severity: ⬛ high · 🟧 medium · 🟨 low · ⬜ info. "Intended" = design evolution, not a gap.

### Grid / layout

| Element | Design says | Code does (file:line) | Verdict | Sev |
|---|---|---|---|---|
| Justified-rows layout | Justified rows (not CSS-column masonry), targetRowHeight 240 desktop / 140 mobile, gap 8 | `CrossAuthorGrid.svelte:61-72` (240/140, boxSpacing 8, AR clamp [0.2,5], 3:2 fallback) | matches / **intended** | ⬜ |
| Load-more affordance | Showcase shows "Load more" + dev cursor footer "remove for production" | `CrossAuthorGrid.svelte:91-104` plain `Load more` button; no cursor footer leaked | matches / **intended** | ⬜ |
| **First-paint SSR tiles** | Public gallery renders full content at first paint without JS (handoff README:193,267-268) | `CrossAuthorGrid.svelte:25,43-60,77-89` — `containerWidth=0` at SSR → `boxes=[]` → zero tiles. Live curl: 0 links/imgs | **gap** | ⬛ |
| Caption overlay | Showcase tile (`showcase-p3.jsx:96-124`) draws gradient + caption always-visible | `CrossAuthorTile.svelte:62-79` — `.cap opacity:0`, revealed only on `:hover`/`:focus-visible` | gap (verify intent; touch has no hover) | 🟧 |
| Tile meta timestamp | `@HANDLE · 2 H AGO` (`showcase-p3.jsx:113-115`) | `CrossAuthorTile.svelte:36-40` renders `@handle` only; `published_at` available but unused | partial | 🟨 |
| Grid horizontal inset | Page rail 64px desktop / 20px mobile (`README:106`; header/pills use 64px) | `CrossAuthorGrid.svelte:107-110` `.grid { margin: 0 32px }` — misaligned with 64px rail | partial | 🟨 |

### Filter rail + discovery header

| Element | Design says | Code does (file:line) | Verdict | Sev |
|---|---|---|---|---|
| Two-group rail + divider, shareable URLs | Sort\|time + category\|following, hairline divider, SSR-first | `FilterPills.svelte:57-131`; `explore/+page.svelte:24-48` `goto(...,{replaceState})`; `+page.server.ts:5-19` reads searchParams | matches / **intended** | ⬜ |
| **Eyebrow frame count** | Site-wide total ("12,418 PUBLISHED FRAMES") | `explore/+page.svelte:85` `photoCount={data.initial.photos.length}` (≤24). True total at `site_stats.rs:14-18` / `SiteStats.frames` | gap (latent; misleads at scale) | ⬛/🟧 |
| "Most discussed" sort | Showcase lists 3 sorts (`showcase-p3.jsx:37`); **spec lists only 2** (`spec:918`) | `FilterPills.svelte:30-33` two sorts; `explore.rs:48,76-77` only `most-appreciated` branch; no `comments_count` column | ambiguous (doc conflict) | 🟨 |
| "Clear" button | Spec: "Clear filters" (plural, `spec:845`); showcase: "Clear" | `FilterPills.svelte:118-128` clears only category, only when a category is set | partial | 🟨 |
| Right-rail accent line | Two lines incl. "● 47 NEW IN LAST 24 HRS" (`showcase-p3.jsx:24-27`) | `DiscoveryHeader.svelte:44-46` renders only hardcoded "NEW MOON IN 6 DAYS" | gap (moon line also hardcoded) | 🟨 |
| "Following only" for anon | No design rule; spec shows toggle unconditionally | `FilterPills.svelte:54,107-116` always shown; backend returns empty page gracefully (`explore.rs:57-69`) | ambiguous (UX call) | 🟨 |

### Tokens / styling

| Element | Design says | Code does (file:line) | Verdict | Sev |
|---|---|---|---|---|
| Core tokens | `styles.css` ladder + amber accent + radii + font stack | `app.css:10,32,64-66,96-97` byte-identical; additive tints/celestial are evolution | matches | ⬜ |
| **Chip radius** | Original handoff: sharp 2px (`handoff/README:121,164-166`); showcase: ~999px pill (`README:123`) | `app.css:430-435` global `.chip { border-radius: var(--r-sm) }` = 2px cascades; discovery chips set none → ships **2px sharp** | resolved → **sharp** (showcase pill line struck 2026-05-30) | ⬜ |
| `var(--bg-faint, …)` live fallback | No `--bg-faint` token; warm ladder is brand | `AladinSkyMap.svelte:140`, `ExternalArchiveLinks.svelte:152`, `TargetIndexCard.svelte:69` — undefined token → cold gray renders | gap | 🟧 |
| Hardcoded `border-radius: 3px` | Radii scale 0/2/4/8/999 | `ExternalArchiveLinks.svelte:154` | gap | 🟨 |
| Stale fallback values (dead) | `--r-md` 4px, `--accent` amber | `AladinSkyMap.svelte:142`, `ExternalArchiveLinks.svelte:98,123,131`, `TargetIndexCard.svelte:41,46` — fallbacks `6px`/`#4a90e2` blue (never trigger, tokens exist) | partial (latent smell) | 🟨 |
| Explore-grid overlay colors | White-on-photo legibility | `CrossAuthorTile.svelte:66,67,92,102`; `AuthorChip.svelte:15,21` — defensible overlay-over-photo | matches | ⬜ |

### Image pipeline

| Element | Design says | Code does (file:line) | Verdict | Sev |
|---|---|---|---|---|
| **Image format (JPEG-only)** | `<Img>` emits no `fm` → JPEG (`aws-s3-cloudfront.md:403-411`) | `Img.svelte:34-35` passes `transform={}` no `fm`; `cdn.ts:10,13-26` `fm` only if present (type is `'auto'\|'jpg'\|'webp'`). All-JPEG, confirmed live (Appendix B) | **JPEG retained** — smallest at every width for noisy astro; WebP larger, AVIF larger or broken (Appendix B). Do not switch, do not add `avif` | ⬜ |
| Modern-format edge fallback | absent/unsupported `fm` should degrade to JPEG | `?fm=avif` at **w≥800** returns a 1 KB `text/html` error page, not a JPEG (Appendix B) | gap — Lambda@Edge errors instead of falling back to JPEG (LOW; infra) | 🟨 |
| **Two-stage LQIP reveal** | 400px blurred → 1200px, 600ms cross-fade (`handoff/README:246,132`) | `Img.svelte:33-43` single bare `<img>`, no placeholder/onload/fade. (Pattern exists in `Photo.svelte:103-115`, not used by explore) | gap | 🟧 |
| blurhash | "blurhash placeholder while loading" (`spec:450,588,762`) | Plumbed to `DiscoveryPhoto.blurhash` but `CrossAuthorTile.svelte:29-35` never passes it; `Img.svelte:42` stashes to `data-` only, never decodes | gap (dead data) | 🟨 |
| fetchpriority | Above-the-fold LCP image should be eager/high | `Img.svelte:38-39` hardcoded `loading="lazy"`, no `fetchpriority`, no `priority` prop. (`Photo.svelte:108-109` has the pattern) | partial | 🟧 |
| Intrinsic dimensions | width/height or aspect-ratio to prevent CLS | `Img.svelte:33-43` no attrs; tile box reserves space via px on `<a>` so explore CLS bounded; other callers unprotected | matches (for explore) | ⬜ |
| CDN URL cache-friendliness | Deterministic w/h/fit/q/fm key | `cdn.ts:13-26`, `Img.svelte:26,32` 1x/2x/3x widths. Caveat: per-tile non-round `w` fragments cache | matches | 🟨 |

### Backend query (`backend/src/discovery/explore.rs`)

| Element | Design says | Code does (file:line) | Verdict | Sev |
|---|---|---|---|---|
| Newest keyset | `(published_at DESC, id DESC)` (`spec:930-931`) | `explore.rs:125,131` row-value `< ($1,$2)` range seek | matches | ⬜ |
| Most-appreciated keyset | 3-tuple `(appreciations_count, published_at, id)` (`spec:932-934`) | `explore.rs:90-92,98,154-158`; cursor encodes apps only on this path | matches | ⬜ |
| One query, no N+1 | `@HANDLE` chip, no avatar | `explore.rs:85-87` single JOIN; `appreciations_count` denormalized | matches | ⬜ |
| Param safety + limit clamp | since whitelist, limit bound | `explore.rs:47,49-55,93-97` clamp [1,60], interval concatenates a *bound int* | matches | ⬜ |
| **Most-appreciated + category index** | Indexes added P1 (`spec:935,948-952`) | No composite; falls back to popular index + category heap filter (`0011:15-17`, `0012:30-32`) | gap — **validate w/ EXPLAIN** | 🟧 (conf: medium) |
| OR-expansion deep pages | Efficient keyset assumed | `explore.rs:90-92` `count<$1 OR (...)` may not collapse to one range cond | partial — **validate w/ EXPLAIN** | 🟨 (conf: medium) |
| Cross-sort cursor edge | Cursors sort-specific | `explore.rs:90` null-apps cursor on popular path → returns page 1 | partial (robustness) | 🟨 |

### a11y / SSR / SEO

| Element | Design / WCAG says | Code does (file:line) | Verdict | Sev |
|---|---|---|---|---|
| Skip-link | "Skip-link to main on every page" (`handoff/README:263`) | None anywhere; `explore/+page.svelte:84` bare `<main>`, `app.html:15-16` none | gap | ⬛ |
| Lightbox focus mgmt | aria-modal + focus trap + restore (`showcase/README:383`) | `Lightbox.svelte:53-59,68` no trap/move-in/restore; `Modal.svelte:19-52` has all three | gap | ⬛ |
| Tile aria-label | "`<target> by @<handle>`" (`showcase/README:384`) | `CrossAuthorTile.svelte:27,33` target only; handle available, omitted | gap | 🟧 |
| prefers-reduced-motion | WCAG 2.3.3 / honoring media query | None (grep 0 hits); `app.css:406-409` scale(1.015) 0.6s unconditional | gap | 🟧 |
| canonical/OG/Twitter | SEO surface (`showcase/README:270`) | `explore/+page.svelte:77-80` title+desc only; `PhotoDetailFull.svelte:370-389` full set | partial | 🟧 |
| Single h1 + theme-flash + per-photo `<a>` exist in component | SSR-first, no flash | `DiscoveryHeader.svelte:42` one h1; `hooks.server.ts:64-74` theme from cookie; `CrossAuthorTile.svelte:22-28` is a real anchor **but gated out of SSR by the layout guard** | h1/theme match; **anchors NOT in SSR HTML** (see §3) | ⬜ / ⬛ |

> **Evidence correction:** one conformance pack asserted "Crawlable per-photo links ARE present in the SSR grid (no gap)." The live curl probe disproves it — both prod and a *populated* staging feed emit `/u/`=0, `/p/`=0. The anchor lives inside `<CrossAuthorTile>`, which the `{#if layout.boxes[i]}` guard (`CrossAuthorGrid.svelte:78-88`) skips entirely when `boxes` is empty. Primary empirical evidence wins; that row is folded into the SSR gap below.

---

## 3. The SSR-empty-grid issue (CONFIRMED)

### Mechanism

`CrossAuthorGrid.svelte` derives the entire layout from `containerWidth = $state(0)` (`:25`), which is assigned **only** inside `onMount` via `getBoundingClientRect()` plus a `ResizeObserver` (`:43-49`). `onMount` does not run during SSR. The `layout` `$derived` short-circuits on `containerWidth <= 0` and returns `{ containerHeight: 0, boxes: [] }` (`:54-60`). The template renders `{#each photos as photo, i}{#if layout.boxes[i]}<CrossAuthorTile .../>{/if}{/each}` (`:77-89`) — with `boxes` empty, the `{#if}` is false for every photo, so **no tile, no `<a href>`, no `<img>`, no caption** is emitted server-side. `EmptyState` does not fire either, because `photos.length` is non-zero (the data is fetched in `+page.server.ts:12-19` and present in the inline hydration payload). The route is SSR-enabled (no `ssr=false`); the data is loaded and then discarded at render time until hydration.

### Empirical confirmation (live probe)

- **Prod** `www.astrophoto.pics/explore`: SSR HTML has `<img`=0, `srcset`=0, `<article`=0, `/u/`=0, `/p/`=0. Grid = `<div class="grid …" style="height:0px">`. The hydration payload *does* carry the 1 published photo (`short_id qc6hl9Pi`).
- **Decisive cross-check** — staging `?sort=newest&since=all` returns **7 real photos from 3 photographers** yet *still* emits 0 tile markers and `height:0px`. This proves the blankness is **structural client-side virtualization**, not an empty-data artifact.
- **First-hand (Appendix A):** the prod LCP element is the H1 *text* — there is no `<img>` for the browser to pick as LCP.

### Consequences

1. **Crawlability.** Googlebot renders JS, so the ~24 SSR-seeded page-1 photos eventually index after the render-queue delay (Google Search Central: "Googlebot queues all pages… for rendering" — https://developers.google.com/search/docs/crawling-indexing/javascript/javascript-seo-basics). The acute harm is **non-rendering consumers**: Bing, social-unfurl scrapers, and LLM crawlers (GPTBot etc.) see zero tiles even on page 1. Everything past page 1 sits behind a `Load more` `<button>` and is uncrawlable by all (Google "doesn't click buttons or scroll").
2. **LCP.** The preload scanner can only discover an LCP image present in the initial HTML response; a JS-injected `<img>` "cannot be discovered by the preload scanner and incurs resource load delay" (web.dev, https://web.dev/articles/optimize-lcp). Today the LCP image is created only after JS + `ResizeObserver` measures the container.
3. **No-JS / first paint.** The feed is a collapsed `height:0` div without JS — contradicting the handoff's "no JS required for first paint" requirement (handoff README:193,268).

### Fix strategies (ranked)

**A — Server-estimate the justified layout (highest fit, lowest effort).** `justifiedLayout()` is a pure function with no DOM dependency and runs unchanged in Node; the *only* thing blocking SSR boxes is the `containerWidth <= 0` guard. Seed `containerWidth` from a sane assumed value during SSR (the `.grid` geometry is known) so real `<a>`/`<img>`/caption tiles ship in raw HTML; the existing `onMount` `ResizeObserver` then re-justifies to the true width on hydration. `p.width`/`p.height` already feed the aspect ratios. Honest trade-off: if the assumed width differs, hydration triggers a re-justify reflow (a CLS event) — mitigate by reserving each tile box with `aspect-ratio` (see B). Source: web.dev LCP guidance above; justified-layout is a pure fn (Flickr lib).

**B — CSS aspect-ratio space reservation as the safety net (pairs with A).** Feed each tile box `aspect-ratio` from the stored `p.width`/`p.height` so vertical space is reserved before image load and a hydration re-justify can't cause a vertical jump. The browser derives the ratio and reserves space before download (JakeArchibald, https://jakearchibald.com/2022/img-aspect-ratio/; web.dev CLS, https://web.dev/articles/optimize-cls). `Img.svelte` already accepts an `aspectRatio` prop — `CrossAuthorTile` just doesn't pass it.

**C — Pure-CSS justified/masonry with container-query units (most crawler-robust, structural).** Express tile widths in `cqw`/`cqi` (`1cqw` = 1% of container inline size — MDN, https://developer.mozilla.org/en-US/docs/Web/CSS/length) so layout needs no measured pixel width at all; identical HTML to every consumer, no hydration reflow. Trade-off: flush last-row justification is approximate vs. Flickr-perfect. Requires `container-type: inline-size` and an `@supports` fallback.

**Plus — crawlable pagination.** Pages 2+ behind the `Load more` `<button>` need real `<a href>` (`?cursor=`/`?page=n`) per Google pagination guidance (rel=next/prev dropped 2019; https://developers.google.com/search/docs/specialty/ecommerce/pagination-and-incremental-page-loading). Stop-gap: the sitemap is currently the only deep path and is capped at `limit=200` — raise the cap / split into a sitemap index until crawlable pagination lands.

---

## 4. Performance improvements (prioritized)

| # | Action | Metric | Effort | Code-level |
|---|---|---|---|---|
| — | **Image format: no action — KEEP JPEG.** Investigated and measured against a real frame at three widths: JPEG is smallest everywhere, WebP is consistently larger (34.0 vs 39.9 KB at w=400), AVIF is larger where it works and broken at the edge for w≥800. Do **not** add an `fm` param and do **not** add `avif` to the `Transform` type. | LCP (download) | — | `Img.svelte:34-35`, `cdn.ts:10,13-26` (no change) |
| P1 | **SSR-render tiles** (see §3-A/B) so the LCP `<img src>` is in raw HTML and preload-scanner-discoverable. | LCP, SEO | M | `CrossAuthorGrid.svelte:25,43-60` |
| P1 | **Add `priority`/eager + `fetchpriority="high"`** prop to `Img.svelte`; `CrossAuthorGrid` passes it to only the first row (mirror `Photo.svelte:108-109`). Never lazy-load the LCP image. | LCP | S | `Img.svelte:38-39` |
| P2 | **Two-stage LQIP reveal:** decode `photo.blurhash` (already plumbed) or a tiny `w=20-40` CDN variant behind the `<img>`, cross-fade on `onload`. Reuse `Photo.svelte:103-115`. | perceived LCP / CLS | M | `Img.svelte`, `CrossAuthorTile.svelte:29-35` |
| P2 | **Correct `sizes`** to the grid's real tile width (avoid 100vw over-fetch) and **snap requested `w` to ~80-120px buckets** to cut CDN cache fragmentation. | LCP, cache hit | S | `cdn.ts`, `CrossAuthorTile.svelte` |
| P2 | **Fonts: de-block the Google Fonts stylesheet.** It's render-blocking, third-party (`fonts.googleapis.com` → `gstatic`), pulls 3 families and is the source of the CLS 0.04 font-swap. Self-host (Fontsource) or `media="print" onload` swap; add `rel=preload` for the latin subsets actually used (preconnect already present). | LCP/FCP, CLS | M | `app.html` head |
| P2 | **`content-visibility:auto` + `contain-intrinsic-size`** on tiles *after* they are SSR-rendered (cuts render of a long feed; web.dev https://web.dev/articles/content-visibility). Never `content-visibility:hidden` (de-indexes). | INP/render | S | `CrossAuthorTile.svelte` |
| P2 | **Backend: add partial composite index** `photos (category, appreciations_count DESC, published_at DESC, id DESC) WHERE published_at IS NOT NULL` for most-appreciated+category — **validate with `EXPLAIN (ANALYZE, BUFFERS)` on a seeded table first** (Docker unreliable in-env; confidence medium). | query latency at scale | M | new migration |
| P2 | **Verify OR-expansion** keyset on deep most-appreciated pages with EXPLAIN; rewrite as row-value comparison if it shows a wide index scan + Filter. Confidence medium, unconfirmed. | query latency | M | `explore.rs:90-92` |
| P3 | **Add `/llms.txt`** — the lone Lighthouse failure (agentic-browsing 67→100); a cheap complement to the crawlability fixes in §3. | agent discoverability | S | static route |

Bundle is fine (~45 immutable-cached HTTP/2 chunks, no blocking issue) — no action. There is **no N+1** in the explore query (single JOIN, denormalized count).

---

## 5. Feature improvements (prioritized, research-grounded)

| Pri | Feature | Rationale + citation |
|---|---|---|
| P1 | **Richer always-visible tile metadata** (target + total integration time as a small persistent caption; reveal scope/camera/filters on hover/focus *with a tap affordance for touch*). | Astro browsers evaluate by acquisition story; AstroBin shows the technical card on the tile. Astrophoto's structured `photo_filters` data is its discovery superpower over Flickr/Unsplash. ForegroundWeb thumbnail-grid guidance (https://www.foregroundweb.com/thumbnail-grids/); AstroBin (https://welcome.astrobin.com/blog/introducing-the-new-astrobin-gallery-experience). Also fixes the current hover-only caption + touch gap. |
| P1 | **Trending sort with time-decay**, default stays Newest. Score `(likes + weighted views)/(age_hrs+2)^gravity`, re-ranked periodically — a young community needs new uploads visible. | 500px Pulse decays after 24h/1wk to give fresh photos a window (https://support.500px.com/hc/en-us/articles/203999378). Flickr interestingness weights engagement *quality* over quantity. |
| P1 | **Faceted catalog search** on integration-time range, scope/camera/filter, target/constellation — serialize all facet+sort state into URL params. | AstroBin's differentiator; URL-encoded state = free "saved searches," shareable, crawlable. NN/g: goal-driven search wants faceted UI + stable URLs, not infinite scroll (https://www.nngroup.com/articles/infinite-scrolling-tips/). |
| P2 | **Three parallel surfaces, not one ranked feed:** Newest (default), Trending, and a manually-flagged "Image of the week." | Every mature platform separates curated / fresh / faceted. Start curation as a single admin flag + badge-on-tile (AstroBin prestige mechanic without the voting funnel). Flickr Explore vs. Following (https://blog.flickr.net/en/2025/09/23/flickr-fundamentals-a-tour-of-flickr-explore/); AstroBin IOTD (https://welcome.astrobin.com/iotd). |
| P2 | **"Most discussed" sort** — only after a `comments_count` denormalized column + composite index exist; reconcile the spec/showcase doc conflict first (see §7). Don't speculate-build. | Behance exposes Most Discussed as a visible toggle (https://help.behance.net/hc/en-us/articles/204484044), but Astrophoto has no discussion data model today. |
| P2 | **Saved collections / boards** ("Galaxies to image", "RedCat 51 inspiration") as an extra discovery surface. | Behance Moodboards recommend fitting work at the board's foot; Unsplash treats Collections as first-class discovery (https://help.are.na/docs/getting-started/connections). |
| P2 | **Hide/disable "Following only" for anonymous users** (or add a sign-in empty-state hint) — selecting it can only ever yield an empty grid. `data.user` is already plumbed via the layout load. | UX polish; backend already degrades gracefully. |
| P3 | **Keyboard grid nav announce + focus move** on Load-more (aria-live "N more frames loaded", focus first new tile). | WCAG 4.1.3 Status Messages (https://www.w3.org/WAI/WCAG22/Understanding/status-messages.html). Load-more button is already the right choice over infinite scroll. |

---

## 6. Accessibility & SEO gaps

| Gap | Citation | Fix |
|---|---|---|
| **No skip-link app-wide** | "Skip-link to main on every page" (handoff/README:263) | Add visually-hidden-until-focused skip-link as first focusable element in `+layout.svelte`/`app.html`; give `<main>` `id="main" tabindex="-1"`. Global fix. |
| **Lightbox: no focus trap / move-in / restore / background-inert** | APG Dialog (Modal) (https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/); Deque focus-restore (https://docs.deque.com/issue-help/1.0.0/en/focus-modal-not-returned/) | `Lightbox.svelte:53-59`: capture `activeElement` on open, move focus to `.close-btn` (tabindex=-1 container), trap Tab, set `inert` on the still-mounted grid behind it (shallow-routed), restore focus on teardown. Reuse `Modal.svelte:19-52`. Gate to modal mode only (it also renders as the standalone permalink). |
| **Tile aria-label omits author** | "`<target> by @<handle>`" (showcase/README:384) | Set link accessible name to `` `${photo.target ?? 'Untitled'} by @${photo.author_handle}` ``; drop the redundant alt+aria double-label (`CrossAuthorTile.svelte:27,33`). |
| **No prefers-reduced-motion** | WCAG 2.3.3 (https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion) | Global `@media (prefers-reduced-motion: reduce)` in `app.css` shortening transitions/removing the `scale(1.015)` hover; keep caption reachable via `:focus-visible`. |
| **No focus outline on tile** | WCAG 2.4.7 / 2.4.13 (https://www.w3.org/WAI/WCAG22/Understanding/focus-appearance.html) | `outline:2px solid var(--accent); outline-offset:2px` on `.tile:focus-visible` and lightbox controls; verify ≥3:1 against the dark backdrop. Today `:focus-visible` only reveals the caption — no ring. |
| **Lightbox close target <24px** | WCAG 2.5.8 Target Size (https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html) | `.close-btn` (`padding:4px 8px`) → min 24×24 (ideally 44×44) hit area. Arrows (44×64) already pass. |
| **Sticky header obscures focused tiles** | WCAG 2.4.11 (https://www.w3.org/WAI/WCAG22/Understanding/focus-not-obscured-minimum.html) | `scroll-padding-top` = AppHeader height on the scroll container. |
| **No canonical/OG/Twitter on /explore** | SEO surface (showcase/README:270) | Add `rel=canonical` (normalize away filter params), `og:*`, `twitter:card`. Pattern in `PhotoDetailFull.svelte:370-389`. |
| **HTML lacks security headers** | Live probe (medium) | No CSP / HSTS / X-Content-Type-Options / X-Frame-Options / Referrer-Policy on the HTML response (prod + staging). Add at the SvelteKit/Koyeb edge. |
| **AVIF edge transform errors instead of falling back (LOW; infra)** | Appendix B (first-hand) | `?fm=avif` at **w≥800** returns a 1 KB `text/html` error page, not an image. The Lambda@Edge origin-request transform should degrade to JPEG on an AVIF failure, never serve an error page. Not blocking — explore ships no `fm` and JPEG is the chosen format — but a latent footgun for any future `fm` caller. Investigate in `docs/operations/aws-s3-cloudfront.md`'s transform path. |

**Do not** add `role="grid"` to the tile gallery — that commits you to a full roving-tabindex/arrow-key model and removes tiles from natural tab order. Wrap the `{#each}` in `<ul role="list">`/`<li>` with a visually-hidden `<h2>` instead (APG Grid pattern, https://www.w3.org/WAI/ARIA/apg/patterns/grid/).

---

## 7. Open design decisions (the two handoffs / spec disagree)

1. **Chip radius — RESOLVED → sharp 2px (2026-05-30).** Original handoff (`handoff/README:121,164-166`) mandates sharp 2px ("Sharp corners signal instrument; no pill shapes except avatars"); the showcase handoff said "chips ~999px pill." Reconciled in favour of **sharp** — it is both the shipped behavior (the global `.chip { border-radius: var(--r-sm) }` at `app.css:430-435`) and the original, token-authoritative handoff. The conflicting "pill" line was struck from `handoff-showcase/README.md`. No code change.
2. **Sort count — 2 vs. 3.** Canonical spec (`spec:918`) lists `newest|most-appreciated`; the showcase mock (`showcase-p3.jsx:37`) lists three including "Most discussed." Backend matches the spec; there is no `comments_count` data model. *Decision:* treat the spec as authoritative (per handoff guidance) and either drop "Most discussed" from the mock or schedule it with the discussion feature — don't speculate-implement.
3. **"Clear" vs. "Clear filters."** Spec wireframe (`spec:845`) says "Clear filters" (plural → reset all); showcase says "Clear." Current code clears only the category and only when one is set. *Decision:* pick the authoritative label, then make the behavior honest.
4. **Tile timestamp on the meta line** (`showcase-p3.jsx:113-115` shows `· N H AGO`) — present in the design, dropped in code. Confirm intentional or restore.
5. **Right-rail second line + moon date** — showcase shows an accent "● N NEW IN LAST 24 HRS"; code shows only a *hardcoded* "NEW MOON IN 6 DAYS." Decide: compute dynamically or descope deliberately (don't ship a hardcoded date as if live).

---

## 8. Prioritized action list

**P0**
- [ ] Make `/explore` SSR-render real tiles (`<a href>`/`<img>`/caption) — seed `containerWidth` for SSR + `aspect-ratio` space reservation; re-justify on hydrate. (`CrossAuthorGrid.svelte:25,43-60`)

**Investigated → JPEG retained (no longer P0)**
- [x] Image format: measured JPEG vs WebP vs AVIF on a real frame at three widths — **JPEG is smallest at every width**. **Decision: keep JPEG**, do not add an `fm` param, do not add `avif` to the `Transform` type. (Appendix B)

**P1**
- [ ] Fix eyebrow count: pass real published-frame total from `/api/site/stats` (widen `photoCount` to bigint/`Number()`). (`explore/+page.svelte:85`)
- [ ] Add `priority`+`fetchpriority="high"` to first-row tiles; stop lazy-loading the LCP image. (`Img.svelte:38-39`)
- [ ] Add app-wide skip-link + `<main id="main" tabindex="-1">`.
- [ ] Lightbox: focus trap + move-in + restore + background `inert`; reuse `Modal.svelte`. Bump close-btn to ≥24px.
- [ ] Fix tile aria-label to `<target> by @<handle>`; remove redundant alt/aria double-label.
- [ ] Add canonical + OG + Twitter meta to `/explore`.
- [x] **Chip-radius decision resolved → sharp**; struck the conflicting "pill" line from `handoff-showcase/README.md`.

**P2**
- [ ] Two-stage LQIP reveal (decode existing `blurhash`); correct `sizes`; snap `w` to width buckets.
- [ ] Global `@media (prefers-reduced-motion: reduce)`; tile `:focus-visible` outline; `scroll-padding-top` for sticky header.
- [ ] De-block Google Fonts (self-host or async swap + preload latin subsets).
- [ ] Validate (EXPLAIN) and add the most-appreciated+category composite index; verify OR-expansion keyset on deep pages. *(confidence medium — unconfirmed without seeded EXPLAIN)*
- [ ] Crawlable `<a href>` pagination behind Load-more; raise/split the sitemap `limit=200` cap as a stop-gap.
- [ ] Align grid inset to 64px rail; restore tile timestamp (or confirm dropped); add/compute the right-rail accent line.
- [ ] Sweep the 3 legacy detail components (`AladinSkyMap`, `ExternalArchiveLinks`, `TargetIndexCard`) for the `--bg-faint`/blue/3px/6px stale-template fallbacks — as a standalone chore, not a feature PR.
- [ ] Add HTML security headers (CSP/HSTS/X-Content-Type-Options/X-Frame-Options) at the edge.
- [ ] Add `/llms.txt` (Lighthouse agentic-browsing fix).

**P3**
- [ ] **(infra, LOW)** AVIF edge transform returns a `text/html` error page at w≥800 instead of falling back to JPEG. Make the Lambda@Edge origin-request transform degrade to JPEG on an AVIF failure. Latent (explore ships no `fm`); fix in `docs/operations/aws-s3-cloudfront.md`'s transform path. (Appendix B)
- [ ] Feature backlog: richer tile metadata, trending+decay sort, faceted catalog search, collections/boards, keyboard load-more announce (see §5).

*Findings marked "validate with EXPLAIN" carry medium confidence and were not empirically confirmed (Docker/testcontainers unreliable in-env). The "Most discussed" sort and chip radius are documented design-source disagreements, not implementation bugs.*

---

## Appendix A — First-hand live measurements (prod, chrome-devtools, 2026-05-30)

> **Caveat:** prod `/explore` has **1 published photo**, so load-time CWV are
> optimistic and *not* representative of a populated grid. They are included for
> the qualitative signals (what the LCP element is, font-swap CLS, header
> hygiene), not as throughput numbers.

**Performance trace** (desktop, 1× CPU, no network throttle, reload):
- **LCP 480 ms** — TTFB 286 ms · load-delay 146 ms · load-duration 3 ms · render-delay 45 ms.
- **CLS 0.04** — attributable to the Google-Fonts `display=swap` font swap.
- **LCP node is the `<h1>` text**, not a photo — there is no `<img>` in the SSR HTML for the browser to choose (direct corroboration of §3).
- Render-blocking insight reported FCP/LCP savings of 0 ms *on this near-empty page* — re-measure on a populated feed before trusting the font de-block sizing.

**Lighthouse** (navigation, desktop, evaluates the **hydrated** DOM):
- Accessibility **100** · Best-Practices **100** · SEO **100** · Agentic-Browsing **67**.
- Only failing audit: **`llms-txt`** (no valid `/llms.txt`).
- ⚠️ **Interpretation:** SEO=100 here means the *post-hydration* DOM is clean; it does **not** clear the no-JS / non-rendering-crawler gap in §3, which Lighthouse cannot see because it runs JS.

**Network** (cold load, explore):
- ~45 immutable-cached JS chunks over HTTP/2 (route node `nodes/15.*.js`) — fine, no blocking concern.
- Google Fonts loaded **third-party**: `fonts.googleapis.com/css2?family=Inter…&family=JetBrains+Mono…&family=Source+Serif+4…&display=swap` → 4 `fonts.gstatic.com` woff2. Render-blocking CSS → font chain across two extra origins; basis for the P2 font de-block.
- Zero image requests (no photos to load) — image-pipeline findings (§4) are necessarily code-derived, not measured here.

---

## Appendix B — CDN format probe (first-hand, 2026-05-30)

`curl -I` against the prod CDN (`https://cdn.astrophoto.pics`) for the real
published photo `dd585183-4689-4207-bb01-49f4f47aa47d` (2657×2665), reproducing
the exact no-`fm` request a `<CrossAuthorTile>` issues:

**Format × width grid** (same frame, explicit `fm`, default `q`):

| Width | JPEG (no `fm` / `fm=jpg`) | `fm=webp` | `fm=avif` |
|---|---|---|---|
| `w=400` | **34.0 KB** | 39.9 KB | 55.9 KB |
| `w=800` | **129 KB** | 141 KB | **ERROR** — `text/html`, ~1 KB |
| `w=1200` | **274 KB** | 287 KB | **ERROR** — `text/html`, ~1 KB |

Plus a no-`fm` negotiation probe: `?w=401` sent with
`Accept: image/avif,image/webp,image/*` still returned **`image/jpeg`** (34.7 KB)
— the CloudFront cache policy strips `Accept`, so no `fm` can ever yield a modern
format.

**Reads:** (1) **JPEG is the smallest format at every width.** For high-noise
astrophotography WebP is consistently **larger** (~+17% at w=400, and larger at
800/1200), and AVIF is *largest* where it works (w=400). This is why the audit
**keeps JPEG** rather than adding an `fm` param. (2) **AVIF is broken at the edge
for w≥800** — the Lambda@Edge transform returns a ~1 KB `text/html` error page
instead of an image (and instead of falling back to JPEG). Tracked as a LOW infra
finding (§6 / §8). This is also why `avif` must **not** be added to the
`Transform` type: the path it would exercise is broken. (3) The edge does not
negotiate on `Accept`, but with JPEG as the chosen output that is moot, not a
defect. The photo-detail SSR page correctly requests `?w=2560` with no `fm`
(`src="https://cdn.astrophoto.pics/img/dd585183-…?w=2560"`) — i.e. JPEG.
