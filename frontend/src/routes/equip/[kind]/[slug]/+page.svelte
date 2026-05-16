<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { untrack } from 'svelte';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import DiscoveryHeader from '$lib/components/discovery/DiscoveryHeader.svelte';
  import FilterPills from '$lib/components/discovery/FilterPills.svelte';
  import CrossAuthorGrid from '$lib/components/discovery/CrossAuthorGrid.svelte';
  import EquipmentPairedRail from '$lib/components/discovery/EquipmentPairedRail.svelte';
  import LightboxHost from '$lib/components/discovery/LightboxHost.svelte';
  import FilterChip from '$lib/components/equipment/FilterChip.svelte';
  import EquipmentMetaCard from '$lib/components/equipment/EquipmentMetaCard.svelte';
  import { fetchEquipmentPage } from '$lib/api/discoveryClient';
  import type { EquipmentSpecsPayload } from '$lib/api/EquipmentSpecsPayload';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  // ── SEO meta ─────────────────────────────────────────────────
  let eq = $derived(data.initial.equipment);
  let eqPhotos = $derived(Number(eq.photo_count));
  let eqLabel = $derived(`${eq.display_name} (${eq.kind})`);
  let eqTitle = $derived(`${eqLabel} — Astrophoto`);
  let eqDescription = $derived(
    `${eqPhotos} astrophotograph${eqPhotos === 1 ? '' : 's'} captured with a ${eq.display_name} on Astrophoto. See what amateur astrophotographers shoot with this ${eq.kind}.`
  );
  let eqCanonical = $derived(
    `${page.url.origin}/equip/${encodeURIComponent(eq.kind)}/${encodeURIComponent(eq.slug)}`
  );

  let cursor = $state<string | null>(untrack(() => data.initial.page.next_cursor));
  $effect(() => {
    cursor = data.initial.page.next_cursor;
  });

  function applyFilter(next: { sort?: string; since?: string; category?: string | undefined }) {
    const u = new URL(window.location.href);
    if (next.sort !== undefined) u.searchParams.set('sort', next.sort);
    if (next.since !== undefined) u.searchParams.set('since', next.since);
    if ('category' in next) {
      if (next.category) {
        u.searchParams.set('category', next.category);
      } else {
        u.searchParams.delete('category');
      }
    }
    void goto(u.pathname + u.search, { replaceState: true, keepFocus: true, noScroll: true });
  }

  async function loadMoreFn() {
    if (!cursor) return { photos: [], next_cursor: null };
    const cur = cursor;
    const result = await fetchEquipmentPage(
      fetch,
      data.initial.equipment.kind,
      data.initial.equipment.slug,
      {
        sort: data.sort,
        since: data.since,
        ...(data.category !== undefined ? { category: data.category } : {}),
        cursor: cur,
        limit: 24
      }
    );
    cursor = result.page.next_cursor;
    return { photos: result.page.photos, next_cursor: result.page.next_cursor };
  }

  // ── Specs helpers ─────────────────────────────────────────────
  const SIZE_LABELS: Record<string, string> = {
    '1_25in': '1.25 inch',
    '2in': '2 inch',
    '31mm': '31 mm',
    '36mm': '36 mm',
    '50mm_round': '50 mm round',
    '50mm_square': '50 mm square',
    other: 'other'
  };

  const TELESCOPE_DESIGN_LABELS: Record<string, string> = {
    refractor_apo: 'Refractor APO',
    refractor_achro: 'Refractor Achro',
    sct: 'SCT',
    rc: 'RC',
    newtonian: 'Newtonian',
    maksutov_cassegrain: 'Maksutov-Cassegrain',
    maksutov_newtonian: 'Maksutov-Newtonian',
    dall_kirkham: 'Dall-Kirkham',
    other: 'Other'
  };

  const MOUNT_TYPE_LABELS: Record<string, string> = {
    equatorial_german: 'EQ German',
    equatorial_fork: 'EQ Fork',
    alt_az: 'Alt-Az',
    harmonic_drive: 'Harmonic Drive',
    strain_wave: 'Strain Wave',
    other: 'Other'
  };

  const FOCAL_MODIFIER_TYPE_LABELS: Record<string, string> = {
    reducer: 'Reducer',
    flattener: 'Flattener',
    reducer_flattener: 'Reducer/Flattener',
    barlow: 'Barlow',
    extender: 'Extender',
    corrector: 'Corrector'
  };

  // The filter chip used by FilterChip.svelte expects a PhotoFilterChip shape.
  // For the filter specs line we build a synthetic chip from the specs.
  let filterChip = $derived.by(() => {
    const specs = data.item?.specs;
    if (!specs || specs.kind !== 'filter') return null;
    return {
      id: data.item?.id ?? '',
      display_name: eq.display_name,
      filter_type: specs.filter_type,
      bandwidth_nm: specs.bandwidth_nm,
      position: 0
    };
  });

  // Build the specs line text based on kind.
  let specsLine = $derived.by((): string | null => {
    const specs: EquipmentSpecsPayload | null | undefined = data.item?.specs;
    if (!specs) return null;
    switch (specs.kind) {
      case 'telescope': {
        const design = specs.design
          ? (TELESCOPE_DESIGN_LABELS[specs.design] ?? specs.design)
          : null;
        const ap = specs.aperture_mm;
        const fl = specs.focal_length_mm;
        const ratio = specs.focal_ratio_f;
        const optics =
          ap != null && fl != null
            ? `${ap}/${fl}${ratio != null ? ` (f/${ratio.toFixed(1)})` : ''}`
            : null;
        return [design, optics].filter(Boolean).join(' ') || null;
      }
      case 'camera': {
        const parts: string[] = [];
        if (specs.sensor_model) parts.push(specs.sensor_model);
        if (specs.color_type) parts.push(specs.color_type === 'mono' ? 'mono' : 'OSC');
        if (specs.cooled != null) parts.push(specs.cooled ? 'cooled' : 'uncooled');
        if (specs.pixel_size_um != null) parts.push(`${specs.pixel_size_um} µm`);
        return parts.length > 0 ? parts.join(' · ') : null;
      }
      case 'mount': {
        const parts: string[] = [];
        if (specs.mount_type) parts.push(MOUNT_TYPE_LABELS[specs.mount_type] ?? specs.mount_type);
        if (specs.payload_kg != null) parts.push(`${specs.payload_kg} kg`);
        if (specs.goto != null) parts.push(specs.goto ? 'GoTo' : 'alt-az manual');
        return parts.length > 0 ? parts.join(' · ') : null;
      }
      case 'focal_modifier': {
        const type = specs.modifier_type
          ? (FOCAL_MODIFIER_TYPE_LABELS[specs.modifier_type] ?? specs.modifier_type)
          : null;
        const factor = specs.factor != null ? `×${specs.factor}` : null;
        return [type, factor].filter(Boolean).join(' ') || null;
      }
      default:
        return null;
    }
  });
</script>

<AppHeader />
<svelte:head>
  <title>{eqTitle}</title>
  <meta name="description" content={eqDescription} />
  <link rel="canonical" href={eqCanonical} />
  <meta property="og:type" content="website" />
  <meta property="og:site_name" content="Astrophoto" />
  <meta property="og:title" content={eqTitle} />
  <meta property="og:description" content={eqDescription} />
  <meta property="og:url" content={eqCanonical} />
  <meta name="twitter:card" content="summary" />
  <meta name="twitter:title" content={eqTitle} />
  <meta name="twitter:description" content={eqDescription} />
</svelte:head>

<DiscoveryHeader variant="equipment" meta={data.initial.equipment} />

{#if data.item}
  <div class="specs-bar">
    {#if data.item.specs === null}
      <a class="t-meta specs-add" href="/equip/{eq.kind}/{eq.slug}/edit">+ Add specs</a>
    {:else if data.item.specs.kind === 'filter'}
      {@const specs = data.item.specs}
      {#if filterChip}
        <FilterChip filter={filterChip} compact />
      {/if}
      {#if specs.bandwidth_nm != null}
        <span class="spec-item"
          ><span class="spec-label">BANDWIDTH</span>&nbsp;{specs.bandwidth_nm} nm</span
        >
      {/if}
      {#if specs.size != null}
        <span class="spec-item"
          ><span class="spec-label">SIZE</span>&nbsp;{SIZE_LABELS[specs.size] ?? specs.size}</span
        >
      {/if}
      {#if specs.mounted != null}
        <span class="spec-item"
          ><span class="spec-label">MOUNTED</span>&nbsp;{specs.mounted ? 'yes' : 'no'}</span
        >
      {/if}
    {:else if specsLine}
      <span class="spec-item spec-line">{specsLine}</span>
    {/if}
  </div>
{/if}

<FilterPills
  variant="equipment"
  sort={data.sort}
  since={data.since}
  {...data.category !== undefined ? { category: data.category } : {}}
  onSortChange={(s) => applyFilter({ sort: s })}
  onSinceChange={(s) => applyFilter({ since: s })}
  onCategoryChange={(c) => applyFilter({ category: c })}
/>
{#key `${data.sort}|${data.since}|${data.category ?? ''}`}
  <CrossAuthorGrid
    initial={{ photos: data.initial.page.photos, next_cursor: data.initial.page.next_cursor }}
    loadMore={loadMoreFn}
  />
{/key}
{#if data.item}
  <EquipmentMetaCard item={data.item} />
{/if}
<EquipmentPairedRail items={data.initial.paired} />
<LightboxHost />
<AppFooter />

<style>
  .specs-bar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 20px;
    padding: 12px 64px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-secondary);
    border-bottom: 1px solid var(--border-subtle);
  }
  .spec-label {
    color: var(--fg-faint);
  }
  .spec-item {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .spec-line {
    color: var(--fg-secondary);
  }
  .specs-add {
    color: var(--accent);
    text-decoration: none;
  }
  .specs-add:hover {
    text-decoration: underline;
  }
  @media (max-width: 768px) {
    .specs-bar {
      padding: 12px 16px;
    }
  }
</style>
