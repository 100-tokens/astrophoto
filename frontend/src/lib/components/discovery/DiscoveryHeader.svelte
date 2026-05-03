<script lang="ts">
  import type { TargetMeta } from '$lib/api/TargetMeta';
  import type { TagMeta } from '$lib/api/TagMeta';
  import type { EquipmentMeta } from '$lib/api/EquipmentMeta';

  type ExploreProps = { variant: 'explore'; photoCount?: number };
  type TargetProps = { variant: 'target'; meta: TargetMeta };
  type TagProps = { variant: 'tag'; meta: TagMeta };
  type EquipmentProps = { variant: 'equipment'; meta: EquipmentMeta };
  type CategoryProps = { variant: 'category'; category: string; photoCount?: bigint };
  type SearchProps = { variant: 'search'; q: string; resultCount?: number };

  type Props =
    | ExploreProps
    | TargetProps
    | TagProps
    | EquipmentProps
    | CategoryProps
    | SearchProps;

  let props: Props = $props();

  const CATEGORY_LABELS: Record<string, string> = {
    dso: 'Deep-Sky Objects',
    planetary: 'Planetary',
    lunar: 'Lunar',
    solar: 'Solar',
    wide_field: 'Wide-field',
    nightscape: 'Nightscape',
    other: 'Other'
  };

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
</script>

{#if props.variant === 'explore'}
  <section class="header header-explore">
    <div class="header-left">
      <p class="eyebrow">● EXPLORE · {props.photoCount ? `${props.photoCount.toLocaleString('en-US')} PUBLISHED FRAMES · ` : ''}UPDATED LIVE</p>
      <h1 class="display">The <em>archive</em>, across photographers</h1>
    </div>
    <div class="header-right">
      <p class="stat-label" style="color: var(--fg-muted);">NEW MOON IN 6 DAYS</p>
      <p class="stat-accent">● 47 NEW IN THE LAST 24 HRS</p>
    </div>
  </section>

{:else if props.variant === 'target'}
  {@const meta = props.meta}
  <section class="header header-target">
    <div class="header-left">
      <p class="eyebrow">● TARGET{meta.kind ? ` · ${meta.kind.toUpperCase()}` : ''}</p>
      <div class="target-title-row">
        <span class="target-slug">{meta.slug.toUpperCase()}</span>
        <h1 class="display">{meta.canonical_name}</h1>
      </div>
      {#if meta.aliases.length > 0}
        <div class="aliases">
          <span class="chip">Also known as</span>
          {#each meta.aliases as alias}
            <span class="chip chip-accent-border">{alias}</span>
          {/each}
        </div>
      {/if}
    </div>
    <div class="stat-block">
      <div class="stat">
        <div class="stat-n stat-n-accent">{fmt(meta.photo_count)}</div>
        <div class="stat-l">PUBLISHED FRAMES</div>
      </div>
      <div class="stat">
        <div class="stat-n">{fmt(meta.contributor_count)}</div>
        <div class="stat-l">CONTRIBUTORS</div>
      </div>
    </div>
  </section>

{:else if props.variant === 'tag'}
  {@const meta = props.meta}
  <section class="header header-tag">
    <div>
      <p class="eyebrow">● TAG</p>
      <h1 class="display"><em>#{meta.name}</em></h1>
      <p class="sub-stat">{fmt(meta.photo_count)} photos tagged</p>
    </div>
  </section>

{:else if props.variant === 'equipment'}
  {@const meta = props.meta}
  <section class="header header-equipment">
    <div class="header-left">
      <p class="eyebrow">
        ● EQUIPMENT · {EQUIPMENT_KIND_LABELS[meta.kind] ?? meta.kind.toUpperCase()} · /EQUIP/{meta.kind.toUpperCase()}/{meta.slug.toUpperCase()}
      </p>
      <h1 class="display display-equipment">{meta.display_name}</h1>
    </div>
    <div class="header-right" style="display: flex; gap: 32px; align-items: flex-end;">
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
        <p class="sub-stat">{fmt(props.photoCount)} photos</p>
      {/if}
    </div>
  </section>

{:else if props.variant === 'search'}
  <section class="header header-search">
    <p class="eyebrow">● SEARCH</p>
    <h1 class="display">Results for <em>"{props.q}"</em></h1>
    {#if props.resultCount !== undefined}
      <p class="sub-stat">{props.resultCount} results</p>
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
    font-size: 48px;
    font-style: italic;
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

  .stat-accent {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.06em;
    color: var(--accent);
    margin: 6px 0 0 0;
  }

  /* Sub-stat (tag/category/search) */
  .sub-stat {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-muted);
    margin: 8px 0 0 0;
  }
</style>
