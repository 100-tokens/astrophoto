<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import Photo from '$lib/components/Photo.svelte';
  import ExifTable from '$lib/components/ExifTable.svelte';
  import type { PhotoDetail } from '$lib/data/photos';

  interface PageData {
    photo: PhotoDetail;
    isRich: boolean;
  }

  let { data }: { data: PageData } = $props();

  let p = $derived(data.photo);
  let isRich = $derived(data.isRich);

  interface ExifRow {
    label: string;
    value: string;
    sublabel?: string;
    valueAccent?: boolean;
  }

  // Build ExifTable rows from the rich detail object.
  // Conditionally spread sublabel to avoid assigning `undefined` to optional
  // properties (exactOptionalPropertyTypes is true in tsconfig).
  let exifRows = $derived<ExifRow[]>(
    isRich
      ? (
          [
            {
              label: 'Target',
              value: p.target,
              ...(p.targetSubtitle ? { sublabel: p.targetSubtitle } : {})
            },
            { label: 'Captured', value: p.captured },
            {
              label: 'Camera',
              value: p.camera,
              ...(p.cameraSub ? { sublabel: p.cameraSub } : {})
            },
            {
              label: 'Telescope',
              value: p.telescope,
              ...(p.telescopeSub ? { sublabel: p.telescopeSub } : {})
            },
            { label: 'Mount', value: p.mount },
            { label: 'Filters', value: p.filters },
            {
              label: 'Exposure',
              value: p.exposure,
              ...(p.exposureTotal ? { sublabel: p.exposureTotal } : {})
            },
            { label: 'Gain', value: p.gain },
            {
              label: 'RA / Dec',
              value: p.ra,
              ...(p.dec ? { sublabel: p.dec } : {})
            },
            { label: 'Field', value: p.field },
            { label: 'Pixel scale', value: p.pixelScale }
          ] as ExifRow[]
        ).filter((r) => r.value)
      : [{ label: 'Record', value: 'Full record not available for this placeholder.' }]
  );

  // Display title: target + optional subtitle
  let titleLine1 = $derived(p.target);
  let titleLine2 = $derived(p.targetSubtitle);
</script>

<AppHeader active="Gallery" />

<!-- Desktop: 2-col grid; mobile: single column -->
<div class="detail-layout">
  <!-- LEFT: image stage -->
  <div class="image-stage">
    <div class="image-inner">
      <Photo target={p.target} style="position: absolute; inset: 0;" />

      <!-- Corner reticles outside the photo (inline, like screens-1.jsx) -->
      <div class="reticle reticle-tl"></div>
      <div class="reticle reticle-tr"></div>
      <div class="reticle reticle-bl"></div>
      <div class="reticle reticle-br"></div>
    </div>

    <!-- Zoom controls -->
    <div class="zoom-controls">
      {#each ['100%', 'fit', '+', '−'] as label}
        <button class="zoom-btn" aria-label="Zoom {label}">{label}</button>
      {/each}
    </div>
  </div>

  <!-- RIGHT: info panel -->
  <aside class="info-panel">
    <div class="info-top">
      {#if isRich && p.publishedDate}
        <div class="t-eyebrow" style="margin-bottom: 12px; color: var(--accent);">
          ● PUBLISHED {p.publishedDate}
        </div>
      {/if}

      <h1 class="photo-title">
        <em>{titleLine1}</em>
        {#if titleLine2}
          <br />{titleLine2}
        {/if}
      </h1>

      <!-- Uploader row -->
      <div class="uploader-row">
        <div class="avatar" aria-hidden="true">
          {p.photographer.initial || p.photographer.name.charAt(0).toUpperCase()}
        </div>
        <div class="uploader-meta">
          <div style="font-weight: 500;">{p.photographer.name}</div>
          {#if isRich}
            <div class="t-meta">
              {p.photographer.frames} frames · Bortle {p.photographer.bortle} · {p.photographer
                .location}
            </div>
          {/if}
        </div>
        <a
          href="/u/{p.photographer.name
            .toLowerCase()
            .replace(/[^a-z0-9]+/g, '-')
            .replace(/^-|-$/g, '')}"
          class="btn btn-secondary btn-sm"
          style="margin-left: auto;"
        >
          Follow
        </a>
      </div>

      {#if p.photographer.caption}
        <p class="caption">{p.photographer.caption}</p>
      {/if}

      <!-- Actions -->
      <div class="action-row">
        <button class="btn btn-secondary btn-sm">♡ {p.appreciations} appreciations</button>
        <button class="btn btn-ghost btn-sm">{p.comments} comments</button>
        <button class="btn btn-ghost btn-sm" style="margin-left: auto;">↗ Share</button>
      </div>
    </div>

    <!-- EXIF section -->
    <div class="exif-section">
      <div class="exif-header">
        <span class="t-label" style="color: var(--fg-primary); letter-spacing: 0.16em;">
          ACQUISITION RECORD
        </span>
        <span class="t-meta">▾</span>
      </div>
      <ExifTable rows={exifRows} />
    </div>
  </aside>
</div>

<!-- Mobile sticky action bar -->
<div class="mobile-actions" aria-label="Photo actions">
  <button class="mobile-action-btn" style="color: var(--accent);">♡ {p.appreciations}</button>
  <button class="mobile-action-btn">💬 {p.comments}</button>
  <button class="mobile-action-btn">↗ Share</button>
</div>

<!-- Footer: hidden on mobile via CSS -->
<div class="desktop-footer">
  <AppFooter />
</div>

<style>
  /* ── Layout ───────────────────────────────────────────────── */
  .detail-layout {
    display: grid;
    grid-template-columns: 1fr 380px;
    min-height: calc(100dvh - 64px);
  }

  /* ── Image stage ──────────────────────────────────────────── */
  .image-stage {
    background: #000;
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 48px;
  }

  .image-inner {
    position: relative;
    width: 100%;
    height: 100%;
    max-height: 780px;
  }

  /* Corner reticles — 14×14 px brackets outside the photo by 8px */
  .reticle {
    position: absolute;
    width: 14px;
    height: 14px;
    pointer-events: none;
    border-color: var(--accent);
    border-style: solid;
  }

  .reticle-tl {
    top: -8px;
    left: -8px;
    border-width: 1px 0 0 1px;
  }

  .reticle-tr {
    top: -8px;
    right: -8px;
    border-width: 1px 1px 0 0;
  }

  .reticle-bl {
    bottom: -8px;
    left: -8px;
    border-width: 0 0 1px 1px;
  }

  .reticle-br {
    bottom: -8px;
    right: -8px;
    border-width: 0 1px 1px 0;
  }

  /* Zoom controls */
  .zoom-controls {
    position: absolute;
    left: 24px;
    bottom: 24px;
    display: flex;
    gap: 4px;
  }

  .zoom-btn {
    width: 32px;
    height: 32px;
    background: rgba(12, 10, 8, 0.7);
    border: 1px solid var(--border-default);
    color: var(--fg-secondary);
    font-family: var(--font-mono);
    font-size: 11px;
    cursor: pointer;
  }

  /* ── Info panel ───────────────────────────────────────────── */
  .info-panel {
    background: var(--bg-base);
    border-left: 1px solid var(--border-subtle);
    overflow-y: auto;
  }

  .info-top {
    padding: 32px;
  }

  .photo-title {
    font-family: var(--font-display);
    font-size: 32px;
    font-weight: 400;
    margin: 0;
    line-height: 1.1;
  }

  .uploader-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--border-subtle);
  }

  .avatar {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: var(--accent);
    color: var(--accent-ink);
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-display);
    font-size: 16px;
    flex-shrink: 0;
  }

  .uploader-meta {
    min-width: 0;
  }

  .caption {
    margin-top: 24px;
    font-size: 14px;
    line-height: 1.65;
    color: var(--fg-secondary);
  }

  .action-row {
    display: flex;
    gap: 8px;
    margin-top: 24px;
  }

  .exif-section {
    padding: 0 32px 32px;
  }

  .exif-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 0;
    border-top: 1px solid var(--border-default);
    border-bottom: 1px solid var(--border-subtle);
    margin-bottom: 8px;
  }

  /* ── Mobile sticky bottom bar ─────────────────────────────── */
  .mobile-actions {
    display: none;
  }

  .mobile-action-btn {
    flex: 1;
    padding: 16px 0;
    color: var(--fg-secondary);
    font-family: var(--font-mono);
    font-size: 12px;
    border: none;
    background: var(--bg-base);
    cursor: pointer;
  }

  .mobile-action-btn + .mobile-action-btn {
    border-left: 1px solid var(--border-subtle);
  }

  /* ── Responsive ───────────────────────────────────────────── */
  @media (max-width: 900px) {
    .detail-layout {
      grid-template-columns: 1fr;
      min-height: auto;
    }

    .image-stage {
      height: 320px;
      padding: 16px;
    }

    .image-inner {
      max-height: 100%;
    }

    .info-panel {
      border-left: none;
      border-top: 1px solid var(--border-subtle);
      overflow-y: visible;
    }

    /* Sticky bottom action bar replaces the desktop action row */
    .mobile-actions {
      display: flex;
      position: sticky;
      bottom: 0;
      border-top: 1px solid var(--border-subtle);
      background: var(--bg-base);
      z-index: 10;
    }

    /* Hide the desktop inline action row on mobile */
    .action-row {
      display: none;
    }

    .desktop-footer {
      display: none;
    }
  }
</style>
