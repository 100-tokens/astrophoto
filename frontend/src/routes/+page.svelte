<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import Button from '$lib/components/Button.svelte';
  import Photo from '$lib/components/Photo.svelte';
  import type { Photo as PhotoData } from '$lib/data/photos';

  interface PageData {
    photos: (PhotoData & { thumbSrc?: string })[];
    heroTarget: string;
    heroTargetSubtitle: string;
    heroIntegration: string;
    heroPhotographer: string;
    heroBortle: number;
  }

  let { data }: { data: PageData } = $props();

  const HEIGHTS = [320, 480, 380, 280, 540, 320, 420, 380, 340, 460, 300, 400];

  const FILTERS = ['All', 'Galaxies', 'Nebulae', 'Solar System', 'Wide field', 'Lunar'];
</script>

<AppHeader active="Gallery" />

<!-- Hero strip -->
<section class="hero">
  <!-- Left column: editorial copy -->
  <div class="hero-copy">
    <div class="t-eyebrow" style="margin-bottom: 16px;">
      <span style="color: var(--accent);">●</span> 14 March 2026 · Friday
    </div>

    <h1 class="hero-h1">
      A quiet archive<br />
      of <em>the night sky</em>,<br />
      kept by those who watch it.
    </h1>

    <p class="hero-body">
      Astrophoto is a home for amateur astrophotographers — a place where an 18-hour integration of
      NGC 7000 looks as monumental as it actually is, and where every frame carries its full record:
      target, equipment, sky.
    </p>

    <div class="hero-actions">
      <Button variant="primary" size="lg" href="/signup">Open an account</Button>
      <Button variant="secondary" size="lg">Browse the gallery →</Button>
    </div>

    <div class="hero-stats">
      <div>
        <span class="stat-num">2,847</span><br />practitioners
      </div>
      <div>
        <span class="stat-num">14,209</span><br />frames
      </div>
      <div>
        <span class="stat-num">11,420 h</span><br />integration
      </div>
    </div>
  </div>

  <!-- Right column: featured photo -->
  <div class="hero-photo-wrap">
    <Photo target={data.heroTarget} style="position: absolute; inset: 0; height: 100%;" />

    <!-- Corner marks (inline — 24×24 at 0 inset) -->
    <div
      style="position: absolute; top: 0; right: 0; width: 24px; height: 24px;
				   border-top: 1px solid var(--accent); border-right: 1px solid var(--accent);"
    ></div>
    <div
      style="position: absolute; bottom: 0; left: 0; width: 24px; height: 24px;
				   border-bottom: 1px solid var(--accent); border-left: 1px solid var(--accent);"
    ></div>

    <!-- Frame of the week tag -->
    <div class="fotw-tag">
      <div style="color: var(--accent);">FRAME OF THE WEEK</div>
      <div style="color: var(--fg-primary); margin-top: 4px;">
        {data.heroTarget} · {data.heroIntegration} SHO
      </div>
      <div style="color: var(--fg-muted);">
        {data.heroPhotographer} · Bortle {data.heroBortle}
      </div>
    </div>
  </div>
</section>

<!-- Filter bar -->
<section class="filter-bar">
  <div class="filter-chips">
    {#each FILTERS as label, i}
      <button class={i === 0 ? 'chip chip-accent' : 'chip'} style="height: 28px; padding: 0 12px;">
        {label}
      </button>
    {/each}
  </div>
  <div class="filter-right">
    <span class="t-label">SORT</span>
    <button class="chip">Newest first ▾</button>
    <span class="t-label" style="margin-left: 12px;">VIEW</span>
    <div class="view-toggle">
      <button class="view-btn active" aria-pressed="true" aria-label="Grid view">▦</button>
      <button class="view-btn" aria-pressed="false" aria-label="List view">≡</button>
    </div>
  </div>
</section>

<!-- Masonry grid -->
<section class="masonry-section">
  <div class="masonry">
    {#each data.photos as photo, i}
      <div class="masonry-item">
        <a href="/photo/{photo.slug}" class="masonry-link" aria-label={photo.target}>
          <div class="photo-wrap" style="height: {HEIGHTS[i % HEIGHTS.length]}px;">
            <Photo
              target={photo.target}
              src={photo.thumbSrc}
              style="position: absolute; inset: 0; height: 100%;"
            />
          </div>
        </a>
        <div class="photo-meta-row">
          <span class="photo-target">{photo.target}</span>
          <span class="photo-integration">{photo.integration}</span>
        </div>
        <div class="photo-photographer">{photo.photographer.toUpperCase()}</div>
      </div>
    {/each}
  </div>
</section>

<!-- Pagination -->
<div class="pagination">
  <Button variant="secondary" size="lg">Load page 2 of 974</Button>
</div>

<AppFooter />

<style>
  /* ── Hero ─────────────────────────────────────────────────── */
  .hero {
    padding: 72px 64px 48px;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 64px;
    align-items: end;
    border-bottom: 1px solid var(--border-subtle);
  }

  .hero-h1 {
    font-family: var(--font-display);
    font-size: 64px;
    line-height: 1.05;
    margin: 0;
    font-weight: 600;
    letter-spacing: -0.015em;
  }

  .hero-body {
    margin-top: 32px;
    font-size: 16px;
    line-height: 1.6;
    color: var(--fg-secondary);
    max-width: 520px;
  }

  .hero-actions {
    margin-top: 32px;
    display: flex;
    gap: 16px;
  }

  .hero-stats {
    margin-top: 48px;
    display: flex;
    gap: 32px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-muted);
  }

  .stat-num {
    color: var(--fg-primary);
    font-size: 20px;
  }

  .hero-photo-wrap {
    position: relative;
    height: 560px;
  }

  .fotw-tag {
    position: absolute;
    left: 16px;
    bottom: 16px;
    background: rgba(12, 10, 8, 0.78);
    padding: 10px 14px;
    border: 1px solid var(--border-default);
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.04em;
  }

  /* ── Filter bar ───────────────────────────────────────────── */
  .filter-bar {
    padding: 24px 64px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-subtle);
  }

  .filter-chips {
    display: flex;
    gap: 8px;
  }

  .filter-right {
    display: flex;
    gap: 16px;
    align-items: center;
  }

  .view-toggle {
    display: flex;
    border: 1px solid var(--border-default);
  }

  .view-btn {
    width: 32px;
    height: 28px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-muted);
    background: none;
    border: none;
    cursor: pointer;
  }

  .view-btn.active {
    background: var(--bg-elevated);
    color: var(--accent);
  }

  /* ── Masonry ──────────────────────────────────────────────── */
  .masonry-section {
    padding: 32px 64px;
  }

  .masonry {
    column-count: 3;
    column-gap: 20px;
  }

  .masonry-item {
    break-inside: avoid;
    margin-bottom: 20px;
  }

  .masonry-link {
    display: block;
  }

  .photo-wrap {
    position: relative;
    overflow: hidden;
  }

  .photo-meta-row {
    display: flex;
    justify-content: space-between;
    padding: 10px 2px;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
  }

  .photo-target {
    color: var(--fg-primary);
  }

  .photo-integration {
    color: var(--fg-muted);
  }

  .photo-photographer {
    padding: 0 2px;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--fg-faint);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  /* ── Pagination ───────────────────────────────────────────── */
  .pagination {
    display: flex;
    justify-content: center;
    padding: 0 0 64px;
  }

  /* ── Responsive ───────────────────────────────────────────── */
  @media (max-width: 900px) {
    .hero {
      grid-template-columns: 1fr;
      padding: 48px 32px 32px;
      gap: 40px;
    }

    .hero-h1 {
      font-size: 44px;
    }

    .hero-photo-wrap {
      height: 320px;
    }

    .filter-bar {
      padding: 16px 32px;
      flex-direction: column;
      align-items: flex-start;
      gap: 12px;
    }

    .masonry-section {
      padding: 24px 32px;
    }

    .masonry {
      column-count: 2;
    }
  }

  @media (max-width: 600px) {
    .masonry {
      column-count: 1;
    }

    .hero {
      padding: 32px 16px 24px;
    }

    .masonry-section {
      padding: 16px;
    }
  }
</style>
