<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import Img from '$lib/components/Img.svelte';
  import type { PhotoDetail } from '$lib/api/types';

  interface PageData {
    photo: PhotoDetail;
    handle: string;
  }

  let { data }: { data: PageData } = $props();
  let p = $derived(data.photo);
  let title = $derived(p.target ?? p.original_name);

  // Compose acquisition rows. Each row is [label, value]; rows with falsy
  // values are filtered out so we don't render empty cells.
  type Row = [string, string | null];
  let acquisition = $derived<Row[]>(
    [
      ['ISO', p.iso != null ? String(p.iso) : null],
      ['Exposure', p.exposure_s != null ? `${p.exposure_s}s` : null],
      ['Focal', p.focal_mm != null ? `${p.focal_mm}mm` : null],
      ['Aperture', p.aperture_f != null ? `f/${p.aperture_f}` : null],
      ['Sessions', p.sessions != null ? String(p.sessions) : null],
      ['Gain', p.gain != null ? String(p.gain) : null],
      ['Sensor temp', p.sensor_temp_c != null ? `${p.sensor_temp_c}°C` : null],
      ['RA', p.ra_deg != null ? `${p.ra_deg.toFixed(4)}°` : null],
      ['Dec', p.dec_deg != null ? `${p.dec_deg.toFixed(4)}°` : null]
    ].filter(([, v]) => v != null) as Row[]
  );
  let equipment = $derived<Row[]>(
    [
      ['Camera', p.camera],
      ['Lens', p.lens]
    ].filter(([, v]) => v != null && v !== '') as Row[]
  );
</script>

<svelte:head>
  <title>{title} — Astrophoto</title>
</svelte:head>

<AppHeader />

<article class="photo-detail">
  <div class="image-wrap">
    <Img photoId={p.id} alt={title} w={1600} sizes="(max-width: 1200px) 100vw, 1200px" />
  </div>

  <div class="info">
    <div class="header-row">
      <div>
        <h1 class="t-display photo-title">{title}</h1>
        <a class="author-link t-mono" href={`/u/${data.handle}`}>@{data.handle.toUpperCase()}</a>
      </div>
    </div>

    {#if p.caption}
      <p class="caption">{p.caption}</p>
    {/if}

    {#if p.tags.length > 0}
      <ul class="tags">
        {#each p.tags as tag}
          <li class="tag-chip"><a href={`/tag/${tag}`}>#{tag}</a></li>
        {/each}
      </ul>
    {/if}

    {#if acquisition.length > 0}
      <section class="meta-block" aria-label="Acquisition">
        <h2 class="t-label">ACQUISITION</h2>
        <dl class="meta-grid">
          {#each acquisition as [label, value]}
            <div class="meta-cell">
              <dt class="t-label">{label}</dt>
              <dd class="t-mono">{value}</dd>
            </div>
          {/each}
        </dl>
      </section>
    {/if}

    {#if equipment.length > 0}
      <section class="meta-block" aria-label="Equipment">
        <h2 class="t-label">EQUIPMENT</h2>
        <dl class="meta-grid">
          {#each equipment as [label, value]}
            <div class="meta-cell">
              <dt class="t-label">{label}</dt>
              <dd class="t-mono">{value}</dd>
            </div>
          {/each}
        </dl>
      </section>
    {/if}
  </div>
</article>

<AppFooter />

<style>
  .photo-detail {
    max-width: 1200px;
    margin: 0 auto;
    padding: 32px 32px 64px;
  }
  .image-wrap {
    width: 100%;
    margin-bottom: 32px;
  }
  .image-wrap :global(img) {
    width: 100%;
    height: auto;
    display: block;
  }
  .info {
    display: flex;
    flex-direction: column;
    gap: 24px;
    max-width: 880px;
  }
  .header-row {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 24px;
  }
  .photo-title {
    font-size: 40px;
    margin: 0 0 8px;
  }
  .author-link {
    color: var(--fg-muted);
    text-decoration: none;
    letter-spacing: 0.08em;
    font-size: 12px;
  }
  .author-link:hover {
    color: var(--accent);
  }
  .caption {
    font-size: 15px;
    line-height: 1.65;
    color: var(--fg-secondary);
    margin: 0;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .tag-chip a {
    display: inline-block;
    padding: 4px 10px;
    border: 1px solid var(--border-subtle);
    color: var(--fg-secondary);
    font-family: var(--font-mono);
    font-size: 11px;
    text-decoration: none;
  }
  .tag-chip a:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
  .meta-block {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .meta-block h2 {
    margin: 0;
    color: var(--fg-muted);
    letter-spacing: 0.12em;
  }
  .meta-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 12px 24px;
    margin: 0;
  }
  .meta-cell {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .meta-cell dt {
    color: var(--fg-muted);
  }
  .meta-cell dd {
    margin: 0;
    color: var(--fg-primary);
    font-size: 13px;
  }
  @media (max-width: 640px) {
    .photo-detail {
      padding: 16px 16px 48px;
    }
    .photo-title {
      font-size: 28px;
    }
  }
</style>
