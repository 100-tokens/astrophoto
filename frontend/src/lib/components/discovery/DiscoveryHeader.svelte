<script lang="ts">
  import type { TargetMeta } from '$lib/api/TargetMeta';
  import type { TagMeta } from '$lib/api/TagMeta';
  import type { EquipmentMeta } from '$lib/api/EquipmentMeta';
  import { pluralize } from '$lib/util/pluralize';
  import { formatRA, formatDec } from '$lib/utils/coords';
  import { objectTypeLabel, constellationLabel } from '$lib/data/celestial';
  import { CATEGORY_LABELS } from '$lib/util/categoryLabel';
  import { daysToNextNewMoon } from '$lib/util/moon';
  import { formatOpposition } from '$lib/util/opposition';

  type ExploreProps = { variant: 'explore'; photoCount?: number | bigint };
  type TargetProps = { variant: 'target'; meta: TargetMeta };
  type TagProps = { variant: 'tag'; meta: TagMeta };
  type EquipmentProps = { variant: 'equipment'; meta: EquipmentMeta };
  type CategoryProps = { variant: 'category'; category: string; photoCount?: number };
  type SearchProps = { variant: 'search'; q: string; resultCount?: number };

  type Props = ExploreProps | TargetProps | TagProps | EquipmentProps | CategoryProps | SearchProps;

  let props: Props = $props();

  const EQUIPMENT_KIND_LABELS: Record<string, string> = {
    telescope: 'Telescope',
    camera: 'Camera',
    mount: 'Mount',
    filter: 'Filters',
    guiding: 'Guiding'
  };

  function fmt(n: bigint | number): string {
    return Number(n).toLocaleString('en-US');
  }

  // Dark-sky planning hint for the explore right rail. Mean lunation, ±1 day.
  const newMoonDays = daysToNextNewMoon();
  const moonLabel =
    newMoonDays <= 0
      ? 'NEW MOON TONIGHT'
      : newMoonDays === 1
        ? 'NEW MOON TOMORROW'
        : `NEW MOON IN ${newMoonDays} DAYS`;
</script>

{#if props.variant === 'explore'}
  <section class="header header-explore">
    <div class="header-left">
      <p class="eyebrow">
        ● EXPLORE · {props.photoCount
          ? `${pluralize(props.photoCount, 'PUBLISHED FRAME').toUpperCase()} · `
          : ''}UPDATED LIVE
      </p>
      <h1 class="display">The <em>archive</em>, across photographers</h1>
    </div>
    <div class="header-right">
      <p class="stat-label" style="color: var(--fg-muted);">{moonLabel}</p>
    </div>
  </section>
{:else if props.variant === 'target'}
  {@const meta = props.meta}
  {@const typeLabel = objectTypeLabel(meta.object_type)}
  {@const constLabel = constellationLabel(meta.constellation)}
  {@const raStr = meta.right_ascension !== null ? formatRA(meta.right_ascension) : ''}
  {@const decStr = meta.declination !== null ? formatDec(meta.declination) : ''}
  {@const sizeStr =
    meta.major_axis_arcmin !== null && meta.minor_axis_arcmin !== null
      ? `${meta.major_axis_arcmin.toFixed(0)}′ × ${meta.minor_axis_arcmin.toFixed(0)}′`
      : ''}
  {@const oppStr = formatOpposition(meta.opposition_doy)}
  <section class="header header-target">
    <div class="header-left">
      <p class="eyebrow">● TARGET{meta.kind ? ` · ${meta.kind.toUpperCase()}` : ''}</p>
      <div class="target-title-row">
        <span class="target-slug">{meta.slug.toUpperCase()}</span>
        <h1 class="display">{meta.canonical_name}</h1>
      </div>
      {#if typeLabel || constLabel || raStr || decStr}
        <p class="meta-line">
          {#if typeLabel}<span>{typeLabel}</span>{/if}
          {#if constLabel}<span> · {constLabel}</span>{/if}
          {#if raStr}<span> · RA {raStr}</span>{/if}
          {#if decStr}<span> · Dec {decStr}</span>{/if}
        </p>
      {/if}
      {#if meta.magnitude_v !== null || sizeStr}
        <p class="meta-line meta-line-secondary">
          {#if meta.magnitude_v !== null}<span>mag {meta.magnitude_v.toFixed(1)}</span>{/if}
          {#if sizeStr}<span>{meta.magnitude_v !== null ? ' · ' : ''}{sizeStr}</span>{/if}
        </p>
      {/if}
      {#if oppStr}
        <p
          class="meta-line meta-line-opposition"
          title="Opposition — the object sits opposite the Sun and transits the meridian at local midnight, its best-observation window. Approximate."
        >
          <span>◐ Opposition · {oppStr}</span>
        </p>
      {/if}
      {#if meta.aliases.length > 0}
        <div class="aliases">
          <span class="chip">Also known as</span>
          {#each meta.aliases as alias}
            <span class="chip chip-accent-border">{alias}</span>
          {/each}
        </div>
      {/if}
      <p class="data-attrib">
        Catalog data:
        <a href="https://github.com/mattiaverga/OpenNGC">OpenNGC by Mattia Verga and contributors</a
        >
        —
        <a href="https://creativecommons.org/licenses/by-sa/4.0/">CC-BY-SA 4.0</a>. Adapted to slug
        format and merged with manual catalog seed.
      </p>
    </div>
    <div class="stat-block">
      <div class="stat">
        <div class="stat-n stat-n-accent">{fmt(meta.photo_count)}</div>
        <div class="stat-l">PUBLISHED {Number(meta.photo_count) === 1 ? 'FRAME' : 'FRAMES'}</div>
      </div>
      <div class="stat">
        <div class="stat-n">{fmt(meta.contributor_count)}</div>
        <div class="stat-l">
          {Number(meta.contributor_count) === 1 ? 'CONTRIBUTOR' : 'CONTRIBUTORS'}
        </div>
      </div>
    </div>
  </section>
{:else if props.variant === 'tag'}
  {@const meta = props.meta}
  <section class="header header-tag">
    <div>
      <p class="eyebrow">● TAG</p>
      <h1 class="display"><em>#{meta.name}</em></h1>
      <p class="sub-stat">{pluralize(meta.photo_count, 'photo')} tagged</p>
    </div>
  </section>
{:else if props.variant === 'equipment'}
  {@const meta = props.meta}
  <section class="header header-equipment">
    <div class="header-left">
      <p class="eyebrow">
        ● {EQUIPMENT_KIND_LABELS[meta.kind] ?? meta.kind.toUpperCase()} · {fmt(meta.photo_count)}
        {Number(meta.photo_count) === 1 ? 'PHOTO' : 'PHOTOS'} IN CATALOG
      </p>
      <h1 class="display display-equipment">{meta.display_name}</h1>
    </div>
    <div class="header-right">
      <div class="header-actions">
        <a class="btn btn-ghost" href="/equip/{meta.kind}/{meta.slug}/edit">Edit specs</a>
        <a class="btn btn-primary" href="/settings/equipment/new">Add to setup</a>
      </div>
      <div class="stat">
        <div class="stat-n">{fmt(meta.photo_count)}</div>
        <div class="stat-l">FRAMES</div>
      </div>
    </div>
  </section>
{:else if props.variant === 'category'}
  <section class="header header-category">
    <div>
      <p class="eyebrow">● CATEGORY</p>
      <h1 class="display">{CATEGORY_LABELS[props.category] ?? props.category}</h1>
      {#if props.photoCount !== undefined}
        <p class="sub-stat">{pluralize(props.photoCount, 'photo')}</p>
      {/if}
    </div>
  </section>
{:else if props.variant === 'search'}
  <section class="header header-search">
    <p class="eyebrow">● SEARCH</p>
    <h1 class="display">Results for <em>"{props.q}"</em></h1>
    {#if props.resultCount !== undefined}
      <p class="sub-stat">{pluralize(props.resultCount, 'result')}</p>
    {/if}
  </section>
{/if}

<style>
  .header {
    padding: 40px 64px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .header-explore,
  .header-target,
  .header-equipment {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 32px;
  }

  .header-left {
    flex: 1;
    min-width: 0;
  }

  .header-right {
    flex-shrink: 0;
    text-align: right;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 16px;
  }

  .header-actions {
    display: flex;
    gap: 8px;
  }

  .eyebrow {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.08em;
    color: var(--fg-muted);
    text-transform: uppercase;
    margin: 0 0 8px 0;
  }

  .display {
    font-family: var(--font-display);
    font-size: 48px;
    font-weight: 600;
    line-height: 1.1;
    color: var(--fg-primary);
    margin: 0;
  }

  .display em {
    font-style: italic;
    color: var(--fg-primary);
  }

  .display-equipment {
    font-size: 64px;
    font-style: italic;
    line-height: 1.05;
  }

  /* Target-specific */
  .target-title-row {
    display: flex;
    align-items: baseline;
    gap: 24px;
    margin-top: 8px;
  }

  .target-slug {
    font-family: var(--font-mono);
    font-size: 64px;
    color: var(--accent);
    letter-spacing: 0.04em;
    line-height: 1;
  }

  .aliases {
    margin-top: 16px;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
  }

  .chip {
    display: inline-block;
    padding: 4px 10px;
    border: 1px solid var(--border-default);
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-secondary);
  }

  .chip-accent-border {
    border-color: var(--accent-dim);
    color: var(--accent);
  }

  /* Stat block */
  .stat-block {
    padding: 24px;
    border: 1px solid var(--border-default);
    background: var(--bg-base);
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    flex-shrink: 0;
    min-width: 280px;
  }

  .stat {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .stat-n {
    font-family: var(--font-display);
    font-size: 32px;
    line-height: 1;
    color: var(--fg-primary);
  }

  .stat-n-accent {
    color: var(--accent);
  }

  .stat-l {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.06em;
    color: var(--fg-muted);
    text-transform: uppercase;
  }

  /* Explore right-side accent */
  .stat-label {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.06em;
    margin: 0;
  }

  /* Sub-stat (tag/category/search) */
  .sub-stat {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-muted);
    margin: 8px 0 0 0;
  }

  /* Target astro meta lines */
  .header-target .meta-line {
    font-size: 0.9rem;
    color: var(--fg-muted);
    margin: 0.25rem 0;
  }
  .header-target .meta-line-secondary {
    font-size: 0.85rem;
  }
  .header-target .meta-line-opposition {
    font-size: 0.85rem;
    color: var(--accent);
    font-family: var(--font-mono);
  }
  .header-target .data-attrib {
    font-size: 0.75rem;
    color: var(--fg-muted);
    margin-top: 1rem;
    opacity: 0.75;
  }
  .header-target .data-attrib a {
    color: inherit;
    text-decoration: underline;
  }

  /* Mobile: tighten padding, stack header rows, downsize displays. */
  @media (max-width: 768px) {
    .header {
      padding: 24px 16px;
    }
    .header-explore,
    .header-target,
    .header-equipment {
      flex-direction: column;
      align-items: stretch;
      gap: 16px;
    }
    .header-right {
      align-items: flex-start;
      text-align: left;
      flex-direction: row;
      justify-content: space-between;
      width: 100%;
    }
    .display {
      font-size: 32px;
    }
    .display-equipment {
      font-size: 36px;
    }
    .target-slug {
      font-size: 36px;
    }
  }
</style>
