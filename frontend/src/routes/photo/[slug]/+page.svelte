<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import MobileHeader from '$lib/components/MobileHeader.svelte';
  import Photo from '$lib/components/Photo.svelte';
  import ExifTable from '$lib/components/ExifTable.svelte';
  import AppreciateButton from '$lib/components/AppreciateButton.svelte';
  import CommentsSection from '$lib/components/CommentsSection.svelte';
  import ReplaceModal from '$lib/components/photos/ReplaceModal.svelte';
  import { invalidateAll } from '$app/navigation';
  import type { PhotoDetail } from '$lib/data/photos';
  import type { Comment } from '$lib/api/client';

  interface ExifRow {
    label: string;
    value: string;
    sublabel?: string;
    valueAccent?: boolean;
    sublabelAccent?: boolean;
  }

  interface PageData {
    photo: PhotoDetail;
    isRich: boolean;
    thumbSrc1200?: string;
    exifRows?: ExifRow[];
    isAppreciated?: boolean;
    comments?: Comment[];
    ownerId?: string;
    current_user_id?: string | null;
    user?: { id: string; displayName: string; following_ids?: string[] } | null;
  }

  let { data }: { data: PageData } = $props();

  let p = $derived(data.photo);
  let isRich = $derived(data.isRich);
  let thumbSrc1200 = $derived(data.thumbSrc1200);

  let isOwner = $derived(
    data.current_user_id != null && data.photo.owner_id === data.current_user_id
  );
  let replaceOpen = $state(false);
  let menuOpen = $state(false);

  function continueHref() {
    const ls = data.photo.last_step;
    if (!ls || ls === 'upload' || ls === 'verify')
      return `/upload/${data.photo.id}/verify`;
    return `/upload/${data.photo.id}/caption`;
  }

  async function discard() {
    if (!confirm('Discard this draft? This cannot be undone.')) return;
    await fetch(`/api/photos/${data.photo.id}`, { method: 'DELETE', credentials: 'include' });
    location.href = '/account/frames';
  }

  function formatRange(a: string, b: string): string {
    const da = new Date(a), db = new Date(b);
    const sameYear = da.getFullYear() === db.getFullYear();
    const fmtShort = { day: '2-digit', month: 'short' } as const;
    const fmtLong = { day: '2-digit', month: 'short', year: 'numeric' } as const;
    return `${da.toLocaleDateString('en-GB', fmtShort).toUpperCase()} → ${db.toLocaleDateString('en-GB', sameYear ? fmtShort : fmtLong).toUpperCase()}`;
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
              ...(p.exposureTotal ? { sublabel: p.exposureTotal, sublabelAccent: true } : {})
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
      : (data.exifRows ?? [])
  );

  // Display title: target + optional subtitle
  let titleLine1 = $derived(p.target);
  let titleLine2 = $derived(p.targetSubtitle);
</script>

<div class="desktop-only"><AppHeader active="Gallery" /></div>
<div class="mobile-only"><MobileHeader backHref="/" /></div>

{#if isOwner && data.photo.is_draft}
  <div class="draft-strip">
    <span class="t-eyebrow accent">● DRAFT · ONLY YOU CAN SEE THIS</span>
    <div class="strip-actions">
      <a href={continueHref()} class="btn btn-secondary btn-sm">Continue editing →</a>
      <button class="btn btn-ghost btn-sm" onclick={discard}>Discard</button>
    </div>
  </div>
{/if}

<!-- Desktop: 2-col grid; mobile: single column -->
<div class="detail-layout">
  <!-- LEFT: image stage -->
  <div class="image-stage">
    <div class="image-inner">
      <Photo target={p.target} src={thumbSrc1200} style="position: absolute; inset: 0;" />

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

      {#if data.photo.replaced_at && data.photo.original_uploaded_at}
        <div class="t-eyebrow muted" style="margin-bottom: 12px;">
          ● REPROCESSED · {formatRange(data.photo.original_uploaded_at, data.photo.replaced_at)}
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
        <p class="caption desktop-caption">{p.photographer.caption}</p>
      {/if}
      {#if p.photographer.captionShort}
        <p class="caption mobile-caption">{p.photographer.captionShort}</p>
      {/if}

      <!-- Actions -->
      <div class="action-row">
        <AppreciateButton
          photoId={p.slug}
          initialCount={p.appreciations}
          initialAppreciated={data.isAppreciated ?? false}
        />
        <button class="btn btn-ghost btn-sm">{p.comments} comments</button>
        <button class="btn btn-ghost btn-sm" style="margin-left: auto;">↗ Share</button>
        {#if isOwner}
          <div class="action-menu">
            <button class="btn btn-ghost btn-sm" onclick={() => (menuOpen = !menuOpen)} aria-label="Actions">⋯</button>
            {#if menuOpen}
              <ul class="menu-popover">
                <li><a href="/upload/{data.photo.id}/verify">Edit metadata</a></li>
                <li><button onclick={() => { replaceOpen = true; menuOpen = false; }}>Replace image…</button></li>
                <li>
                  {#if data.photo.is_draft}
                    <button onclick={discard}>Discard draft</button>
                  {:else}
                    <button onclick={discard}>Delete photo</button>
                  {/if}
                </li>
              </ul>
            {/if}
          </div>
        {/if}
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

    {#if data.comments != null && data.ownerId != null}
      <CommentsSection
        photoOwnerId={data.ownerId}
        comments={data.comments}
        currentUser={data.user ?? null}
      />
    {/if}
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

{#if data.photo.id}
  <ReplaceModal bind:open={replaceOpen} photoId={data.photo.id} onreplaced={() => invalidateAll()} />
{/if}

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

  /* Visibility helpers */
  .desktop-only {
    display: block;
  }
  .mobile-only {
    display: none;
  }
  .mobile-caption {
    display: none;
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

  /* ── Responsive (≤768px = phone) ─────────────────────────── */
  @media (max-width: 768px) {
    .desktop-only {
      display: none;
    }
    .mobile-only {
      display: block;
    }

    .detail-layout {
      grid-template-columns: 1fr;
      min-height: auto;
    }

    .image-stage {
      height: 320px;
      padding: 0;
    }

    .image-inner {
      max-height: 100%;
    }

    /* Strip technical chrome from the photo on mobile per the design */
    .reticle,
    .zoom-controls {
      display: none;
    }

    .info-panel {
      border-left: none;
      border-top: 0;
      overflow-y: visible;
    }

    .info-top {
      padding: 20px 20px 0;
    }

    .photo-title {
      font-size: 28px;
    }

    .desktop-caption {
      display: none;
    }
    .mobile-caption {
      display: block;
      margin-top: 16px;
      font-size: 13px;
    }

    /* EXIF table shrinks to 11px on mobile */
    .exif-section {
      padding: 0 20px 20px;
      font-size: 11px;
    }
    .exif-section :global(.exif),
    .exif-section :global(.exif th),
    .exif-section :global(.exif td) {
      font-size: 11px;
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

    .action-row {
      display: none;
    }

    .desktop-footer {
      display: none;
    }
  }

  /* ── Draft strip ──────────────────────────────────────────── */
  .draft-strip {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 64px;
    background: rgba(208, 160, 80, 0.08);
    border-bottom: 1px solid var(--warning, #c0a060);
  }

  .draft-strip .accent {
    color: var(--accent);
  }

  .strip-actions {
    display: flex;
    gap: 8px;
  }

  /* ── Owner ⋯ menu ─────────────────────────────────────────── */
  .action-menu {
    position: relative;
    display: inline-block;
  }

  .menu-popover {
    position: absolute;
    top: 100%;
    right: 0;
    background: var(--bg-canvas);
    border: 1px solid var(--border-default);
    list-style: none;
    padding: 4px 0;
    min-width: 180px;
    z-index: 10;
    margin: 0;
  }

  .menu-popover li > a,
  .menu-popover li > button {
    display: block;
    width: 100%;
    text-align: left;
    padding: 8px 16px;
    background: none;
    border: none;
    color: var(--fg-primary);
    cursor: pointer;
    text-decoration: none;
    font-size: 14px;
  }

  .menu-popover li > a:hover,
  .menu-popover li > button:hover {
    background: var(--bg-raised);
  }

  /* ── REPROCESSED / muted eyebrow ──────────────────────────── */
  .muted {
    color: var(--fg-muted);
  }
</style>
