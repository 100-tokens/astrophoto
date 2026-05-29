<script lang="ts">
  import { untrack } from 'svelte';
  import { page } from '$app/state';
  import { invalidateAll } from '$app/navigation';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import Img from '$lib/components/Img.svelte';
  import ZoomableImage from '$lib/components/photos/ZoomableImage.svelte';
  import AppreciateButton from '$lib/components/AppreciateButton.svelte';
  import CommentThread from '$lib/components/photos/CommentThread.svelte';
  import ReplaceModal from '$lib/components/photos/ReplaceModal.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import FilterChip from '$lib/components/equipment/FilterChip.svelte';
  import '$lib/components/equipment/filter-chip.css';
  import { rowTotalS, grandTotalS, totalSubs, formatHm } from '$lib/utils/integration';
  import ProcessingPipeline from './ProcessingPipeline.svelte';
  import CelestialOverlay from '$lib/components/celestial/CelestialOverlay.svelte';
  import CelestialPanel from '$lib/components/celestial/CelestialPanel.svelte';
  import type { PhotoDetail, ProcessingReport } from '$lib/api/types';
  import type { GalleryPhoto } from '$lib/api/GalleryPhoto';
  import type { CelestialObject } from '$lib/api/CelestialObject';

  async function share() {
    if (typeof navigator === 'undefined') return;
    if (navigator.share) {
      try {
        await navigator.share({ url: location.href });
      } catch {
        /* user cancelled */
      }
    } else {
      try {
        await navigator.clipboard.writeText(location.href);
      } catch {
        /* clipboard unavailable */
      }
    }
  }

  interface PageData {
    photo: PhotoDetail;
    handle: string;
    morePhotos: GalleryPhoto[];
    processing?: ProcessingReport | null;
    celestialObjects?: CelestialObject[];
    platesolveStatus?: import('$lib/api/PlatesolveStatus').PlatesolveStatus | null;
  }

  let { data }: { data: PageData } = $props();
  let p = $derived(data.photo);

  // Celestial overlay state — survives within this page but resets on
  // navigation. Layer set defaults to all OpenNGC object_type families
  // we render, with PGC behind its own toggle so the default view is
  // not drowned by faint galaxies.
  let selectedSlug = $state<string | null>(null);
  let layers = $state<Set<string>>(
    new Set(['G', 'Neb', 'OCl', 'GCl', 'PN', 'HII', 'SNR', 'Cl+N'])
  );
  let showPgc = $state(false);
  // Labels on by default so the object name is visible without a click;
  // the "labels" pill in the panel can toggle them off if the frame is busy.
  let labelsAlwaysOn = $state(true);
  let celestial = $derived<CelestialObject[]>(data.celestialObjects ?? []);
  let solveForOverlay = $derived(
    data.platesolveStatus?.state === 'solved' &&
      data.platesolveStatus.raDeg != null &&
      data.platesolveStatus.decDeg != null &&
      data.platesolveStatus.pixelScaleArcsec != null &&
      p.width != null &&
      p.height != null
      ? {
          raDeg: data.platesolveStatus.raDeg,
          decDeg: data.platesolveStatus.decDeg,
          pixelScaleArcsec: data.platesolveStatus.pixelScaleArcsec,
          rotationDeg: data.platesolveStatus.rotationDeg ?? 0,
          width: p.width,
          height: p.height
        }
      : null
  );

  // Owner mode — Camille viewing her own M42 photo gets the Edit /
  // Replace / Delete affordances. Anyone else just sees the read-only
  // action row.
  let viewer = $derived(page.data.user);
  let isOwner = $derived(viewer != null && viewer.id === p.owner_id);
  let replaceOpen = $state(false);
  let deleteOpen = $state(false);
  let deleteError = $state<string | null>(null);

  // Live comment count — seeded from p.comment_count, then driven by the
  // CommentThread's oncountchange callback so the action-row label stays in
  // sync with optimistic post/delete inside the thread. Re-seeds on
  // navigation when p.comment_count changes (different photo).
  let liveCommentCount = $state(untrack(() => Number(p.comment_count)));
  $effect(() => {
    liveCommentCount = Number(p.comment_count);
  });

  async function performDelete() {
    deleteError = null;
    try {
      const r = await fetch(`/api/photos/${p.id}`, {
        method: 'DELETE',
        credentials: 'include'
      });
      if (!r.ok) {
        deleteError = `Delete failed: ${r.status}`;
        return;
      }
      deleteOpen = false;
      // Land back on the photographer's profile after delete.
      window.location.href = `/u/${data.handle}`;
    } catch (e) {
      deleteError = `Delete failed: ${(e as Error).message}`;
    }
  }

  // Title treatment per showcase-p2 ScreenLightbox: split target on first
  // whitespace if it parses as "<catalog-id> <common-name>" so the catalog
  // id renders as italic display, and the common name follows. Otherwise
  // render the target as a single italic Display.
  let title = $derived(p.target ?? p.original_name);
  let titleHead = $derived.by(() => {
    if (!p.target) return null;
    const m = p.target.match(/^([^ ]+)\s+(.+)$/);
    return m ? { head: m[1], rest: m[2] } : null;
  });

  // Eyebrow: "● PUBLISHED <date>" in accent. published_at isn't on PhotoDetail
  // yet, so derive from created_at which is close enough for now.
  let publishedDate = $derived(
    new Date(p.created_at)
      .toLocaleDateString('en-GB', {
        day: '2-digit',
        month: 'short',
        year: 'numeric'
      })
      .toUpperCase()
  );

  // Acquisition rows — show what's set, drop the rest. Two-line cells where
  // there's a derived value (Exposure → "180 × 360 s" + "= 18.0 hours" accent).
  type Row = { label: string; value: string; sub: string | undefined; subAccent: boolean };
  let acquisitionRows = $derived.by<Row[]>(() => {
    const rows: Row[] = [];
    const row = (label: string, value: string, sub?: string, subAccent = false): Row => ({
      label,
      value,
      sub,
      subAccent
    });
    if (p.target) rows.push(row('Target', p.target));
    if (p.taken_at) {
      const d = new Date(p.taken_at).toLocaleDateString('en-GB', {
        day: '2-digit',
        month: 'short',
        year: 'numeric'
      });
      rows.push(
        row(
          'Captured',
          d,
          p.sessions ? `${p.sessions} session${p.sessions === 1 ? '' : 's'}` : undefined
        )
      );
    } else if (p.sessions != null) {
      rows.push(row('Sessions', String(p.sessions)));
    }
    if (p.camera) rows.push(row('Camera', p.camera));
    if (p.lens) rows.push(row('Lens', p.lens));
    if (p.exposure_s != null) {
      const total = p.sessions ? p.exposure_s * p.sessions : null;
      const totalLabel = total != null ? formatDuration(total) : undefined;
      rows.push(
        row(
          'Exposure',
          `${p.exposure_s} s${p.sessions ? ` × ${p.sessions}` : ''}`,
          totalLabel ? `= ${totalLabel}` : undefined,
          true
        )
      );
    }
    if (p.focal_mm != null) {
      const apt = p.aperture_f != null ? ` · f/${p.aperture_f}` : '';
      rows.push(row('Focal', `${p.focal_mm} mm${apt}`));
    } else if (p.aperture_f != null) {
      rows.push(row('Aperture', `f/${p.aperture_f}`));
    }
    if (p.iso != null) rows.push(row('ISO', String(p.iso)));
    if (p.gain != null) rows.push(row('Gain', String(p.gain)));
    if (p.sensor_temp_c != null) rows.push(row('Sensor', `${p.sensor_temp_c} °C`));
    if (p.ra_deg != null && p.dec_deg != null) {
      rows.push(row('RA / Dec', formatCoords(p.ra_deg, p.dec_deg)));
    }

    // Enrich from the XISF observation summary (parsed locally from the
    // header). Additive — only fields the columns above don't already cover.
    // Site coordinates are intentionally NOT surfaced (privacy: the header
    // embeds precise GPS; it's parsed/stored but not shown publicly).
    const obs = data.processing?.observation;
    if (obs) {
      if (obs.telescope) rows.push(row('Telescope', obs.telescope));
      const noFilters = (p.filter_items?.length ?? 0) === 0 && !p.filters;
      if (obs.filter && noFilters) rows.push(row('Filter', obs.filter));
      if (obs.pixelScaleArcsec != null) {
        const bin = obs.binning && obs.binning > 1 ? ` · bin ${obs.binning}×${obs.binning}` : '';
        rows.push(row('Pixel scale', `${obs.pixelScaleArcsec.toFixed(2)}″/px${bin}`));
      }
      if (obs.observationStart && obs.observationEnd) {
        const d = (s: string) =>
          new Date(s).toLocaleDateString('en-GB', {
            day: '2-digit',
            month: 'short',
            year: 'numeric'
          });
        const start = d(obs.observationStart);
        const end = d(obs.observationEnd);
        rows.push(row('Acquisition window', start === end ? start : `${start} → ${end}`));
      }
    }
    return rows;
  });

  function formatDuration(seconds: number): string {
    const h = Math.floor(seconds / 3600);
    const m = Math.round((seconds % 3600) / 60);
    if (h === 0) return `${m} min`;
    if (m === 0) return `${h} h`;
    return `${h} h ${m.toString().padStart(2, '0')} m`;
  }
  function formatCoords(ra: number, dec: number): string {
    const raH = Math.floor(ra / 15);
    const raM = Math.floor((ra / 15 - raH) * 60);
    const raS = Math.round(((ra / 15 - raH) * 60 - raM) * 60);
    const sign = dec >= 0 ? '+' : '-';
    const abs = Math.abs(dec);
    const decD = Math.floor(abs);
    const decM = Math.floor((abs - decD) * 60);
    const decS = Math.round(((abs - decD) * 60 - decM) * 60);
    return `${raH}ʰ ${raM}ᵐ ${raS}ˢ · ${sign}${decD}° ${decM}′ ${decS}″`;
  }

  // ── Filter chips ───────────────────────────────────────────────
  const chips = $derived(p.filter_items ?? []);
  const cacheTokens = $derived(
    (p.filters ?? '')
      .split(',')
      .map((t: string) => t.trim())
      .filter(Boolean)
  );
  const known = $derived(new Set(chips.map((c) => c.display_name)));
  const orphans = $derived(cacheTokens.filter((t) => !known.has(t)));

  // ── SEO / GEO meta ─────────────────────────────────────────────
  const CDN_BASE: string = (import.meta.env.VITE_CDN_BASE_URL as string | undefined) ?? '';

  // Description: 160-char-ish summary built from caption or composed from
  // the acquisition record. Search engines truncate around 160; AI/LLMs
  // read the lot.
  let metaDescription = $derived.by(() => {
    if (p.caption) return p.caption.slice(0, 280);
    const bits = [
      p.target ?? p.original_name,
      p.exposure_s != null && p.sessions != null
        ? `${formatDuration(p.exposure_s * p.sessions)} integration`
        : null,
      p.camera ? `shot on ${p.camera}` : null,
      `by @${data.handle}`
    ].filter(Boolean);
    return bits.join(' · ');
  });

  // Always render the absolute /u/<handle>/p/<short_id> form as canonical
  // even when reached via a /photo/<uuid> redirect or alias.
  let canonicalUrl = $derived(
    `${page.url.origin}/u/${encodeURIComponent(data.handle)}/p/${p.short_id}`
  );

  // Display master at 1200 px makes a good rich-unfurl preview without
  // blowing the social-card weight budget. The CDN handles the resize.
  let ogImage = $derived(
    CDN_BASE ? `${CDN_BASE}/img/${p.id}?w=1200` : `${page.url.origin}/api/photos/${p.id}/thumb/1200`
  );
  // og:image:width/height must match what the URL actually serves, not
  // the original capture dims, or unfurl clients reflow on first paint.
  // We request w=1200 so the served dims are 1200 × (1200 × h/w).
  const OG_IMAGE_WIDTH = 1200;
  let ogImageHeight = $derived.by(() => {
    if (p.width == null || p.height == null || p.width === 0) return null;
    return Math.round(OG_IMAGE_WIDTH * (p.height / p.width));
  });

  // schema.org Photograph — the structured data search engines and AI
  // crawlers actually consume. Photograph has the best fit for a single
  // astrophotograph; we attach the photographer as creator (Person) and
  // surface the target name + capture metadata as keywords/exif so a
  // query like "M31 18-hour integration" surfaces this URL.
  let jsonLd = $derived.by(() => {
    const integrationSeconds =
      p.exposure_s != null && p.sessions != null ? p.exposure_s * p.sessions : null;
    const keywords = [
      p.target,
      ...(p.tags ?? []),
      p.camera,
      p.lens,
      p.target ? 'astrophotography' : null
    ].filter(Boolean);
    const obj: Record<string, unknown> = {
      '@context': 'https://schema.org',
      '@type': 'Photograph',
      '@id': canonicalUrl,
      name: title,
      description: metaDescription,
      url: canonicalUrl,
      contentUrl: ogImage,
      thumbnailUrl: ogImage,
      datePublished: p.created_at,
      keywords: keywords.join(', '),
      creator: {
        '@type': 'Person',
        name: `@${data.handle}`,
        url: `${page.url.origin}/u/${data.handle}`
      },
      copyrightHolder: {
        '@type': 'Person',
        url: `${page.url.origin}/u/${data.handle}`
      },
      width: p.width,
      height: p.height,
      // EXIF subset — schema.org doesn't have a first-class astrophoto
      // type, so we use additionalProperty for capture metadata.
      additionalProperty: [
        p.iso != null ? { '@type': 'PropertyValue', name: 'iso', value: p.iso } : null,
        p.exposure_s != null
          ? { '@type': 'PropertyValue', name: 'exposureSeconds', value: p.exposure_s }
          : null,
        p.focal_mm != null
          ? { '@type': 'PropertyValue', name: 'focalLengthMm', value: p.focal_mm }
          : null,
        p.aperture_f != null
          ? { '@type': 'PropertyValue', name: 'apertureFNumber', value: p.aperture_f }
          : null,
        integrationSeconds != null
          ? {
              '@type': 'PropertyValue',
              name: 'totalIntegrationSeconds',
              value: integrationSeconds
            }
          : null,
        p.target ? { '@type': 'PropertyValue', name: 'target', value: p.target } : null,
        p.ra_deg != null
          ? { '@type': 'PropertyValue', name: 'rightAscensionDeg', value: p.ra_deg }
          : null,
        p.dec_deg != null
          ? { '@type': 'PropertyValue', name: 'declinationDeg', value: p.dec_deg }
          : null
      ].filter(Boolean)
    };
    if (p.target) {
      obj.about = { '@type': 'Thing', name: p.target };
    }
    return JSON.stringify(obj).replace(/</g, '\\u003c');
  });
</script>

<svelte:head>
  <title>{title} — Astrophoto</title>
  <meta name="description" content={metaDescription} />
  <link rel="canonical" href={canonicalUrl} />

  <!-- Open Graph for rich link unfurls (Slack/Discord/Bluesky/etc.) -->
  <meta property="og:type" content="article" />
  <meta property="og:site_name" content="Astrophoto" />
  <meta property="og:title" content={`${title} — Astrophoto`} />
  <meta property="og:description" content={metaDescription} />
  <meta property="og:url" content={canonicalUrl} />
  <meta property="og:image" content={ogImage} />
  <meta property="og:image:alt" content={title} />
  {#if ogImageHeight != null}
    <meta property="og:image:width" content={String(OG_IMAGE_WIDTH)} />
    <meta property="og:image:height" content={String(ogImageHeight)} />
  {/if}

  <!-- Twitter -->
  <meta name="twitter:card" content="summary_large_image" />
  <meta name="twitter:title" content={`${title} — Astrophoto`} />
  <meta name="twitter:description" content={metaDescription} />
  <meta name="twitter:image" content={ogImage} />

  <!-- schema.org Photograph + ImageObject + Person — the structured data
       AI engines and Google Image actually consume. -->
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html `<script type="application/ld+json">${jsonLd}</script>`}
</svelte:head>

<AppHeader />

<main>
  <article class="detail">
    <!-- Image stage: full-bleed black, ratio held by the photo -->
    <div class="stage">
      <div class="stage-frame">
        <ZoomableImage photoId={p.id} alt={title} w={2560} maxHeight="calc(100dvh - 64px - 96px)">
          {#snippet overlay(zoomScale)}
            {#if celestial.length > 0 && solveForOverlay}
              <CelestialOverlay
                objects={celestial}
                solve={solveForOverlay}
                {layers}
                {showPgc}
                {labelsAlwaysOn}
                {zoomScale}
                bind:selectedSlug
                onSelect={(slug) => (selectedSlug = slug)}
              />
            {/if}
          {/snippet}
        </ZoomableImage>
        <!-- Corner reticles, accent-colored, per the spec -->
        <span class="reticle reticle-tl" aria-hidden="true"></span>
        <span class="reticle reticle-tr" aria-hidden="true"></span>
        <span class="reticle reticle-bl" aria-hidden="true"></span>
        <span class="reticle reticle-br" aria-hidden="true"></span>
      </div>
    </div>

    <!-- Info aside, 380px right column -->
    <aside class="info">
      <div class="info-inner">
        <div class="t-eyebrow accent">● PUBLISHED {publishedDate}</div>

        <h1 class="title">
          {#if titleHead}
            <em>{titleHead.head}</em>
            <br />{titleHead.rest}
          {:else}
            {title}
          {/if}
        </h1>

        <div class="author-row">
          <div class="avatar" aria-hidden="true">{(data.handle[0] ?? 'U').toUpperCase()}</div>
          <div class="author-meta">
            <a class="author-name" href={`/u/${data.handle}`}>@{data.handle}</a>
          </div>
          <a class="btn btn-secondary btn-sm" href={`/u/${data.handle}`}>View profile</a>
        </div>

        {#if p.caption}
          <p class="caption">{p.caption}</p>
        {/if}

        {#if p.tags.length > 0}
          <ul class="tags">
            {#each p.tags as tag}
              <li><a class="chip" href={`/tag/${tag}`}>#{tag}</a></li>
            {/each}
          </ul>
        {/if}

        {#if chips.length > 0 || orphans.length > 0}
          <div class="filter-strip-head">
            <span class="t-label">FILTERS</span>
            <span class="t-meta"
              >{chips.length} TYPED{orphans.length > 0 ? ` · ${orphans.length} LEGACY` : ''}</span
            >
          </div>
          <div class="filter-strip">
            {#each chips as f (f.id)}<FilterChip filter={f} />{/each}
            {#each orphans as tok}
              <span class="fchip-orphan"><span class="lbl">legacy</span>{tok}</span>
            {/each}
          </div>
        {/if}

        {#if p.filter_integrations?.length}
          <div class="filter-strip-head">
            <span class="t-label">INTEGRATION</span>
            <span class="t-meta"
              >{formatHm(grandTotalS(p.filter_integrations))} · {totalSubs(
                p.filter_integrations
              )} subs</span
            >
          </div>
          <ul class="fi-strip">
            {#each p.filter_integrations as r, i (i)}
              <li class="fi-item">
                <span class="fi-band">{r.filter || '—'}</span>
                <span class="t-meta fi-detail"
                  >{r.sub_count} × {r.sub_exposure_s} s{#if r.gain != null} · gain {r.gain}{/if}{#if r.sensor_temp_c != null}
                    · {r.sensor_temp_c} °C{/if}</span
                >
                <span class="fi-rt t-meta">{formatHm(rowTotalS(r))}</span>
              </li>
            {/each}
          </ul>
        {/if}

        <div class="actions">
          <AppreciateButton photoId={p.id} initialCount={Number(p.appreciation_count)} />
          <a class="btn btn-ghost btn-sm" href="#comments"
            >{liveCommentCount} comment{liveCommentCount === 1 ? '' : 's'}</a
          >
          <button type="button" class="btn btn-ghost btn-sm action-share" onclick={share}
            >↗ Share</button
          >
          {#if isOwner}
            <span class="action-divider" aria-hidden="true">·</span>
            <a class="btn btn-ghost btn-sm" href={`/upload/${p.id}/verify`}>✏ Edit</a>
            <button type="button" class="btn btn-ghost btn-sm" onclick={() => (replaceOpen = true)}
              >↻ Replace</button
            >
            <button
              type="button"
              class="btn btn-ghost btn-sm action-delete"
              onclick={() => {
                deleteError = null;
                deleteOpen = true;
              }}>× Delete</button
            >
          {/if}
        </div>

        {#if acquisitionRows.length > 0}
          <div class="acquisition-header">
            <span class="t-label">ACQUISITION RECORD</span>
          </div>
          <table class="exif">
            <tbody>
              {#each acquisitionRows as row}
                <tr>
                  <th>{row.label}</th>
                  <td>
                    {row.value}
                    {#if row.sub}
                      <br /><span class={row.subAccent ? 'sub accent' : 'sub'}>{row.sub}</span>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}

        {#if data.processing}
          <ProcessingPipeline report={data.processing} />
        {/if}

        {#if celestial.length > 0 && solveForOverlay}
          <CelestialPanel
            objects={celestial}
            bind:selectedSlug
            bind:layers
            bind:showPgc
            bind:labelsAlwaysOn
            {isOwner}
            photoId={p.id}
          />
        {/if}

        <CommentThread
          photoId={p.id}
          photoOwnerId={p.owner_id}
          initialCount={Number(p.comment_count)}
          oncountchange={(n) => (liveCommentCount = n)}
        />

        {#if data.morePhotos.length > 0}
          <div class="more-header"><span class="t-label">MORE FROM @{data.handle}</span></div>
          <div class="more-grid">
            {#each data.morePhotos.slice(0, 4) as mp}
              <a class="more-tile" href={`/u/${data.handle}/p/${mp.short_id}`}>
                <Img photoId={mp.id} alt={mp.target ?? ''} w={300} />
              </a>
            {/each}
          </div>
        {/if}
      </div>
    </aside>
  </article>
</main>

{#if isOwner}
  <ReplaceModal
    bind:open={replaceOpen}
    photoId={p.id}
    onreplaced={() => {
      replaceOpen = false;
      void invalidateAll();
    }}
  />
  <ConfirmDialog
    bind:open={deleteOpen}
    title="Delete photo"
    message={deleteError ??
      `Delete "${title}"? This removes the photo permanently and cannot be undone.`}
    confirmLabel={deleteError ? 'Retry' : 'Delete'}
    tone="danger"
    onconfirm={performDelete}
  />
{/if}

<AppFooter />

<style>
  .detail {
    display: grid;
    grid-template-columns: 1fr 380px;
    align-items: start;
    min-height: calc(100dvh - 64px);
  }
  .stage {
    background: #000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 48px;
    position: sticky;
    top: 64px;
  }
  .stage-frame {
    position: relative;
    width: 100%;
    max-width: 100%;
    /* ZoomableImage owns image sizing (max-height passed as a prop). */
  }
  .reticle {
    position: absolute;
    width: 14px;
    height: 14px;
    pointer-events: none;
  }
  .reticle-tl {
    top: -8px;
    left: -8px;
    border-top: 1px solid var(--accent);
    border-left: 1px solid var(--accent);
  }
  .reticle-tr {
    top: -8px;
    right: -8px;
    border-top: 1px solid var(--accent);
    border-right: 1px solid var(--accent);
  }
  .reticle-bl {
    bottom: -8px;
    left: -8px;
    border-bottom: 1px solid var(--accent);
    border-left: 1px solid var(--accent);
  }
  .reticle-br {
    bottom: -8px;
    right: -8px;
    border-bottom: 1px solid var(--accent);
    border-right: 1px solid var(--accent);
  }

  .info {
    background: var(--bg-base);
    border-left: 1px solid var(--border-subtle);
    overflow-y: auto;
  }
  .info-inner {
    padding: 32px;
  }
  .title {
    font-family: var(--font-display);
    font-size: 32px;
    font-weight: 400;
    margin: 12px 0 0;
    line-height: 1.1;
  }
  .title em {
    font-style: italic;
  }

  .author-row {
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
  .author-meta {
    flex: 1;
    min-width: 0;
  }
  .author-name {
    font-weight: 500;
    color: var(--fg-primary);
    text-decoration: none;
  }
  .author-name:hover {
    color: var(--accent);
  }

  .caption {
    margin: 24px 0 0;
    font-size: 14px;
    line-height: 1.65;
    color: var(--fg-secondary);
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    list-style: none;
    margin: 16px 0 0;
    padding: 0;
  }
  .tags .chip {
    display: inline-block;
    text-decoration: none;
    font-size: 11px;
    padding: 3px 8px;
  }

  .action-divider {
    color: var(--fg-faint);
    align-self: center;
    margin: 0 4px;
  }
  .action-delete:hover {
    color: var(--danger);
    border-color: var(--danger);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 24px;
  }
  .action-share {
    margin-left: auto;
  }
  /* The auto-margin on .action-share is a no-op when the row wraps
     (the buttons land on a new line); flex-wrap is the safety net so
     owner actions (Replace, Delete) don't clip on tablet widths. */

  .filter-strip-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    padding: 16px 0 8px;
    border-top: 1px solid var(--border-default);
    margin-top: 24px;
  }
  .fi-strip {
    list-style: none;
    margin: 0;
    padding: 0 0 16px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .fi-item {
    display: grid;
    grid-template-columns: 3rem 1fr auto;
    align-items: baseline;
    gap: 12px;
  }
  .fi-band {
    font-family: var(--font-mono);
    font-weight: 600;
    color: var(--fg-primary);
  }
  .fi-rt {
    color: var(--fg-secondary);
    white-space: nowrap;
  }
  .filter-strip {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    padding-bottom: 16px;
  }

  .acquisition-header,
  .more-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 0;
    border-top: 1px solid var(--border-default);
    border-bottom: 1px solid var(--border-subtle);
    margin-top: 24px;
  }
  .acquisition-header .t-label {
    color: var(--fg-primary);
    letter-spacing: 0.16em;
  }

  .exif {
    margin-top: 8px;
  }
  .exif .sub {
    color: var(--fg-muted);
  }
  .exif .sub.accent {
    color: var(--accent);
  }

  .more-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
    margin-top: 12px;
  }
  .more-tile {
    position: relative;
    aspect-ratio: 1 / 1;
    overflow: hidden;
    background: var(--bg-elevated);
    display: block;
  }
  .more-tile :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  /* Stack on narrow viewports — image on top, panel below. */
  @media (max-width: 960px) {
    .detail {
      grid-template-columns: 1fr;
    }
    .info {
      border-left: 0;
      border-top: 1px solid var(--border-subtle);
    }
    .stage {
      padding: 16px;
      position: static;
    }
    .more-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>
