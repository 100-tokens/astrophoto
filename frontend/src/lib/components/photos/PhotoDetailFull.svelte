<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import Img from '$lib/components/Img.svelte';
  import AppreciateButton from '$lib/components/AppreciateButton.svelte';
  import CommentThread from '$lib/components/photos/CommentThread.svelte';
  import type { PhotoDetail } from '$lib/api/types';
  import type { GalleryPhoto } from '$lib/api/GalleryPhoto';

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
  }

  let { data }: { data: PageData } = $props();
  let p = $derived(data.photo);

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
      rows.push(row('Captured', d, p.sessions ? `${p.sessions} sessions` : undefined));
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
</script>

<svelte:head>
  <title>{title} — Astrophoto</title>
</svelte:head>

<AppHeader />

<article class="detail">
  <!-- Image stage: full-bleed black, ratio held by the photo -->
  <div class="stage">
    <div class="stage-frame">
      <Img
        photoId={p.id}
        alt={title}
        w={2400}
        sizes="(max-width: 1200px) 100vw, calc(100vw - 380px)"
      />
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

      <div class="actions">
        <AppreciateButton photoId={p.id} initialCount={Number(p.appreciation_count)} />
        <a class="btn btn-ghost btn-sm" href="#comments">{p.comment_count} comments</a>
        <button type="button" class="btn btn-ghost btn-sm action-share" onclick={share}
          >↗ Share</button
        >
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

      <CommentThread
        photoId={p.id}
        photoOwnerId={p.owner_id}
        initialCount={Number(p.comment_count)}
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

<AppFooter />

<style>
  .detail {
    display: grid;
    grid-template-columns: 1fr 380px;
    min-height: calc(100dvh - 64px);
  }
  .stage {
    background: #000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 48px;
    position: relative;
  }
  .stage-frame {
    position: relative;
    width: 100%;
    max-width: 100%;
  }
  .stage-frame :global(img) {
    width: 100%;
    height: auto;
    display: block;
    max-height: calc(100dvh - 64px - 96px);
    object-fit: contain;
    margin: 0 auto;
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

  .actions {
    display: flex;
    gap: 8px;
    margin-top: 24px;
  }
  .action-share {
    margin-left: auto;
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
    }
    .stage-frame :global(img) {
      max-height: 70vh;
    }
    .more-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>
