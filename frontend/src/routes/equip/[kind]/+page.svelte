<script lang="ts">
  import { goto } from '$app/navigation';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import type { EquipmentItemDetail } from '$lib/api/EquipmentItemDetail';
  import type { EquipmentFacetBucket } from '$lib/api/EquipmentFacetBucket';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  // ── Per-kind display labels ────────────────────────────────────
  const KIND_LABELS: Record<string, string> = {
    telescope: 'Telescopes',
    camera: 'Cameras',
    mount: 'Mounts',
    filter: 'Filters',
    focal_modifier: 'Focal modifiers',
    guiding: 'Guiding'
  };

  const DESIGN_LABELS: Record<string, string> = {
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
    equatorial_german: 'Equatorial — German',
    equatorial_fork: 'Equatorial — Fork',
    alt_az: 'Alt-Az',
    harmonic_drive: 'Harmonic drive',
    strain_wave: 'Strain wave',
    other: 'Other'
  };

  const FILTER_TYPE_LABELS: Record<string, string> = {
    luminance: 'Luminance',
    red: 'Red',
    green: 'Green',
    blue: 'Blue',
    h_alpha: 'H-alpha',
    oiii: 'OIII',
    sii: 'SII',
    uv_ir_cut: 'UV/IR cut',
    dual_band: 'Dual band',
    tri_band: 'Tri band',
    quad_band: 'Quad band',
    light_pollution: 'Light pollution',
    broadband_color: 'Broadband color',
    other: 'Other'
  };

  const FOCAL_MODIFIER_LABELS: Record<string, string> = {
    reducer: 'Reducer',
    flattener: 'Flattener',
    reducer_flattener: 'Reducer/Flattener',
    barlow: 'Barlow',
    extender: 'Extender',
    corrector: 'Corrector'
  };

  const GUIDING_SETUP_LABELS: Record<string, string> = {
    oag: 'OAG',
    guidescope: 'Guidescope',
    oag_prism: 'OAG prism',
    other: 'Other'
  };

  // Derived view state — recomputed when SvelteKit navigation refreshes
  // `data` (URL → server load → new payload). No client-side caching:
  // the URL is the source of truth for every filter / sort / page.
  let items = $derived(data.response.items);
  let facets = $derived(data.response.facets);
  let total = $derived(Number(data.response.total));
  let kindLabel = $derived(KIND_LABELS[data.kind] ?? data.kind);
  let selectedBrands = $derived(new Set((data.brand ?? '').split(',').filter(Boolean)));

  // ── Helpers ─────────────────────────────────────────────────────
  function applyParam(updates: Record<string, string | undefined>) {
    const u = new URL(window.location.href);
    for (const [k, v] of Object.entries(updates)) {
      if (v === undefined || v === '') u.searchParams.delete(k);
      else u.searchParams.set(k, v);
    }
    // Reset to page 0 whenever filters change so the user doesn't
    // land on a now-empty page.
    if (!('page' in updates)) u.searchParams.delete('page');
    void goto(u.pathname + u.search, { replaceState: true, keepFocus: true, noScroll: true });
  }

  function toggleBrand(name: string) {
    const next = new Set(selectedBrands);
    if (next.has(name)) next.delete(name);
    else next.add(name);
    const joined = Array.from(next).join(',');
    applyParam({ brand: joined || undefined });
  }

  let searchDebounce: ReturnType<typeof setTimeout> | undefined;
  function onSearchInput(v: string) {
    if (searchDebounce) clearTimeout(searchDebounce);
    searchDebounce = setTimeout(() => applyParam({ q: v.trim() || undefined }), 200);
  }

  function clearAll() {
    void goto(`/equip/${data.kind}`, { replaceState: true, keepFocus: true });
  }

  function slugFor(item: EquipmentItemDetail): string {
    return item.canonical_name.replace(/\s+/g, '-');
  }

  /**
   * Build the per-item spec summary shown in each grid card. Mirrors
   * the per-kind logic the existing detail page uses for its specs
   * line, kept inline so the card is self-contained.
   */
  function specsSummary(it: EquipmentItemDetail): string {
    const s = it.specs;
    if (!s) return '—';
    switch (s.kind) {
      case 'telescope': {
        const ap = s.aperture_mm;
        const fl = s.focal_length_mm;
        const ratio = s.focal_ratio_f;
        const optics =
          ap != null && fl != null
            ? `${ap}/${fl}${ratio != null ? ` (f/${ratio.toFixed(1)})` : ''} mm`
            : '';
        const design = s.design ? (DESIGN_LABELS[s.design] ?? s.design) : '';
        return [design, optics].filter(Boolean).join(' · ') || '—';
      }
      case 'camera': {
        const parts: string[] = [];
        if (s.sensor_model) parts.push(s.sensor_model);
        if (s.color_type) parts.push(s.color_type === 'mono' ? 'mono' : 'OSC');
        if (s.cooled != null) parts.push(s.cooled ? 'cooled' : 'uncooled');
        if (s.pixel_size_um != null) parts.push(`${s.pixel_size_um} µm`);
        return parts.join(' · ') || '—';
      }
      case 'mount': {
        const parts: string[] = [];
        if (s.mount_type) parts.push(MOUNT_TYPE_LABELS[s.mount_type] ?? s.mount_type);
        if (s.payload_kg != null) parts.push(`${s.payload_kg} kg payload`);
        if (s.goto != null && s.goto) parts.push('GoTo');
        return parts.join(' · ') || '—';
      }
      case 'filter': {
        const parts: string[] = [];
        if (s.filter_type) parts.push(FILTER_TYPE_LABELS[s.filter_type] ?? s.filter_type);
        if (s.bandwidth_nm != null) parts.push(`${s.bandwidth_nm} nm`);
        if (s.size) parts.push(s.size.replace('_', ' '));
        return parts.join(' · ') || '—';
      }
      case 'focal_modifier': {
        const parts: string[] = [];
        if (s.modifier_type) parts.push(FOCAL_MODIFIER_LABELS[s.modifier_type] ?? s.modifier_type);
        if (s.factor != null) parts.push(`×${s.factor}`);
        return parts.join(' · ') || '—';
      }
      case 'guiding': {
        const parts: string[] = [];
        if (s.setup_kind) parts.push(GUIDING_SETUP_LABELS[s.setup_kind] ?? s.setup_kind);
        if (s.guide_focal_mm != null) parts.push(`${s.guide_focal_mm} mm`);
        return parts.join(' · ') || '—';
      }
      default:
        return '—';
    }
  }

  function bucketLabel(facetKey: string, b: EquipmentFacetBucket): string {
    switch (facetKey) {
      case 'designs':
        return DESIGN_LABELS[b.value] ?? b.value;
      case 'mount_types':
        return MOUNT_TYPE_LABELS[b.value] ?? b.value;
      case 'filter_types':
        return FILTER_TYPE_LABELS[b.value] ?? b.value;
      case 'modifier_types':
        return FOCAL_MODIFIER_LABELS[b.value] ?? b.value;
      case 'setup_kinds':
        return GUIDING_SETUP_LABELS[b.value] ?? b.value;
      case 'sensor_types':
        return b.value.toUpperCase();
      case 'color_types':
        return b.value === 'mono' ? 'Mono' : b.value === 'osc' ? 'OSC' : b.value;
      case 'cooled':
        return b.value === 'yes' ? 'Cooled' : 'Uncooled';
      default:
        return b.value;
    }
  }

  function hasActiveFilters(): boolean {
    return Boolean(
      data.q || data.brand || data.minAperture || data.maxAperture || data.sort !== 'most_used'
    );
  }
</script>

<svelte:head>
  <title>{kindLabel} — Equipment catalog — Astrophoto</title>
  <meta
    name="description"
    content={`Browse the Astrophoto community catalog of ${kindLabel.toLowerCase()}. Filter by brand, sort by aperture or popularity.`}
  />
</svelte:head>

<AppHeader />

<main class="catalog">
  <header class="hdr">
    <p class="t-eyebrow accent">Catalog · {data.kind.replace('_', ' ').toUpperCase()}</p>
    <h1 class="t-display">{kindLabel}</h1>
    <p class="t-meta">
      {total.toLocaleString('en-US')} item{total === 1 ? '' : 's'} in the catalog
    </p>
  </header>

  <div class="toolbar">
    <input
      class="search"
      type="search"
      placeholder="Search brand or model…"
      aria-label="Search"
      value={data.q ?? ''}
      oninput={(e) => onSearchInput((e.target as HTMLInputElement).value)}
    />
    <label class="sort">
      <span class="t-meta">SORT</span>
      <select
        value={data.sort}
        onchange={(e) => applyParam({ sort: (e.target as HTMLSelectElement).value })}
      >
        <option value="most_used">Most used</option>
        <option value="brand_asc">Brand A → Z</option>
        {#if data.kind === 'telescope'}
          <option value="aperture_desc">Aperture ↓</option>
        {/if}
        <option value="recent">Most recent</option>
      </select>
    </label>
  </div>

  <div class="layout">
    <aside class="sidebar" aria-label="Filters">
      <section class="facet">
        <h2 class="t-eyebrow">BRAND</h2>
        {#if facets.brands.length === 0}
          <p class="t-meta empty">No brands match.</p>
        {:else}
          <ul class="checkboxes">
            {#each facets.brands as b (b.value)}
              <li>
                <label>
                  <input
                    type="checkbox"
                    checked={selectedBrands.has(b.value)}
                    onchange={() => toggleBrand(b.value)}
                  />
                  <span class="name">{b.value}</span>
                  <span class="count">{b.count}</span>
                </label>
              </li>
            {/each}
          </ul>
        {/if}
      </section>

      {#if data.kind === 'telescope'}
        <section class="facet">
          <h2 class="t-eyebrow">APERTURE (mm)</h2>
          <div class="range-pair">
            <input
              type="number"
              min="20"
              max="600"
              placeholder="min"
              value={data.minAperture ?? ''}
              onchange={(e) =>
                applyParam({ min_aperture: (e.target as HTMLInputElement).value || undefined })}
            />
            <span class="dash">—</span>
            <input
              type="number"
              min="20"
              max="600"
              placeholder="max"
              value={data.maxAperture ?? ''}
              onchange={(e) =>
                applyParam({ max_aperture: (e.target as HTMLInputElement).value || undefined })}
            />
          </div>
        </section>
        {#if facets.designs && facets.designs.length > 0}
          <section class="facet">
            <h2 class="t-eyebrow">DESIGN</h2>
            <ul class="checkboxes readonly">
              {#each facets.designs as b (b.value)}
                <li class="readonly-row">
                  <span class="name">{bucketLabel('designs', b)}</span>
                  <span class="count">{b.count}</span>
                </li>
              {/each}
            </ul>
          </section>
        {/if}
      {/if}

      {#if data.kind === 'camera'}
        {#each [{ key: 'sensor_types' as const, label: 'SENSOR' }, { key: 'color_types' as const, label: 'COLOR' }, { key: 'cooled' as const, label: 'COOLED' }] as f (f.key)}
          {@const buckets = facets[f.key]}
          {#if buckets && buckets.length > 0}
            <section class="facet">
              <h2 class="t-eyebrow">{f.label}</h2>
              <ul class="checkboxes readonly">
                {#each buckets as b (b.value)}
                  <li class="readonly-row">
                    <span class="name">{bucketLabel(f.key, b)}</span>
                    <span class="count">{b.count}</span>
                  </li>
                {/each}
              </ul>
            </section>
          {/if}
        {/each}
      {/if}

      {#if data.kind === 'mount' && facets.mount_types && facets.mount_types.length > 0}
        <section class="facet">
          <h2 class="t-eyebrow">TYPE</h2>
          <ul class="checkboxes readonly">
            {#each facets.mount_types as b (b.value)}
              <li class="readonly-row">
                <span class="name">{bucketLabel('mount_types', b)}</span>
                <span class="count">{b.count}</span>
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      {#if data.kind === 'filter' && facets.filter_types && facets.filter_types.length > 0}
        <section class="facet">
          <h2 class="t-eyebrow">FILTER TYPE</h2>
          <ul class="checkboxes readonly">
            {#each facets.filter_types as b (b.value)}
              <li class="readonly-row">
                <span class="name">{bucketLabel('filter_types', b)}</span>
                <span class="count">{b.count}</span>
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      {#if data.kind === 'focal_modifier' && facets.modifier_types && facets.modifier_types.length > 0}
        <section class="facet">
          <h2 class="t-eyebrow">TYPE</h2>
          <ul class="checkboxes readonly">
            {#each facets.modifier_types as b (b.value)}
              <li class="readonly-row">
                <span class="name">{bucketLabel('modifier_types', b)}</span>
                <span class="count">{b.count}</span>
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      {#if data.kind === 'guiding' && facets.setup_kinds && facets.setup_kinds.length > 0}
        <section class="facet">
          <h2 class="t-eyebrow">SETUP KIND</h2>
          <ul class="checkboxes readonly">
            {#each facets.setup_kinds as b (b.value)}
              <li class="readonly-row">
                <span class="name">{bucketLabel('setup_kinds', b)}</span>
                <span class="count">{b.count}</span>
              </li>
            {/each}
          </ul>
        </section>
      {/if}

      {#if hasActiveFilters()}
        <button type="button" class="clear-link" onclick={clearAll}>Clear filters</button>
      {/if}
    </aside>

    <section class="grid-pane">
      {#if items.length === 0}
        <p class="empty-state t-meta">
          No catalog items match your filters.
          {#if hasActiveFilters()}
            <button type="button" class="inline-clear" onclick={clearAll}>Clear filters</button>
          {/if}
        </p>
      {:else}
        <ul class="grid">
          {#each items as it (it.id)}
            <li>
              <a class="card" href="/equip/{data.kind}/{slugFor(it)}">
                <div class="card-thumb" aria-hidden="true">
                  <!-- Catalog items are abstract — no real image is associated.
                       A monogram bubble + accent dot stands in for a thumb so the
                       grid keeps a consistent rhythm without dummy artwork. -->
                  <span class="monogram">{(it.brand || it.model || '?').charAt(0)}</span>
                </div>
                <div class="card-body">
                  <p class="t-eyebrow">{(it.brand || 'Unknown').toUpperCase()}</p>
                  <h3 class="card-title t-display t-display-i">
                    {it.model}{it.variant ? ` · ${it.variant}` : ''}
                  </h3>
                  <p class="card-specs t-meta">{specsSummary(it)}</p>
                </div>
                <div class="card-meta">
                  <span class="usage" title={`Used by ${it.usage_count} photo(s)`}>
                    <span class="usage-dot" aria-hidden="true"></span>
                    {it.usage_count} photo{it.usage_count === 1 ? '' : 's'}
                  </span>
                </div>
              </a>
            </li>
          {/each}
        </ul>
        {#if total > 24}
          <nav class="pager t-meta" aria-label="Pagination">
            {#if data.page > 0}
              <button type="button" onclick={() => applyParam({ page: String(data.page - 1) })}
                >← Previous</button
              >
            {/if}
            <span class="pager-state"
              >Page {data.page + 1} of {Math.max(1, Math.ceil(total / 24))}</span
            >
            {#if (data.page + 1) * 24 < total}
              <button type="button" onclick={() => applyParam({ page: String(data.page + 1) })}
                >Next →</button
              >
            {/if}
          </nav>
        {/if}
      {/if}
    </section>
  </div>
</main>

<AppFooter />

<style>
  .catalog {
    max-width: 1440px;
    margin: 0 auto;
    padding: 32px 64px 64px;
    color: var(--fg-primary);
  }
  .hdr {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 24px;
  }
  .hdr h1 {
    margin: 0;
    font-size: clamp(2.5rem, 4vw, 3.5rem);
  }
  .toolbar {
    display: flex;
    gap: 16px;
    align-items: center;
    margin-bottom: 24px;
    padding-bottom: 16px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .search {
    flex: 1 1 320px;
    max-width: 480px;
    background: var(--bg-raised);
    border: 1px solid var(--border-default);
    color: var(--fg-primary);
    font-family: var(--font-ui);
    font-size: 14px;
    padding: 10px 14px;
  }
  .search:focus {
    outline: none;
    border-color: var(--accent);
  }
  .sort {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .sort select {
    background: var(--bg-raised);
    border: 1px solid var(--border-default);
    color: var(--fg-primary);
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 6px 10px;
  }
  .layout {
    display: grid;
    grid-template-columns: 280px minmax(0, 1fr);
    gap: 40px;
    align-items: start;
  }
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 28px;
    position: sticky;
    top: 24px;
  }
  .facet {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .facet h2 {
    margin: 0;
  }
  .checkboxes {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 280px;
    overflow-y: auto;
  }
  .checkboxes label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    padding: 4px 0;
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--fg-secondary);
  }
  .checkboxes label:hover {
    color: var(--fg-primary);
  }
  .checkboxes .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .checkboxes .count {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
  }
  .checkboxes input[type='checkbox'] {
    accent-color: var(--accent);
  }
  .readonly-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 0;
    font-family: var(--font-ui);
    font-size: 13px;
    color: var(--fg-muted);
  }
  .range-pair {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .range-pair input {
    width: 5.5rem;
    background: var(--bg-raised);
    border: 1px solid var(--border-default);
    color: var(--fg-primary);
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 6px 8px;
  }
  .range-pair .dash {
    color: var(--fg-muted);
  }
  .clear-link {
    background: none;
    border: none;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: 12px;
    text-align: left;
    padding: 0;
    cursor: pointer;
  }
  .clear-link:hover {
    text-decoration: underline;
  }
  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 16px;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 20px;
    background: var(--bg-raised);
    border: 1px solid var(--border-default);
    text-decoration: none;
    color: var(--fg-primary);
    transition:
      border-color 0.12s,
      transform 0.12s;
    height: 100%;
  }
  .card:hover {
    border-color: var(--accent);
    transform: translateY(-1px);
  }
  .card-thumb {
    width: 100%;
    aspect-ratio: 16 / 9;
    background: var(--bg-elevated);
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border-subtle);
  }
  .monogram {
    font-family: var(--font-display);
    font-size: 48px;
    font-weight: 500;
    color: var(--fg-muted);
    text-transform: uppercase;
  }
  .card-body {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
  }
  .card-body p {
    margin: 0;
  }
  .card-title {
    font-size: 20px;
    line-height: 1.15;
    margin: 0;
  }
  .card-specs {
    color: var(--fg-muted);
  }
  .card-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 8px;
    border-top: 1px solid var(--border-subtle);
  }
  .usage {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
  }
  .usage-dot {
    width: 6px;
    height: 6px;
    background: var(--accent);
    border-radius: 50%;
  }
  .pager {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-top: 24px;
    padding-top: 16px;
    border-top: 1px solid var(--border-subtle);
  }
  .pager button {
    background: none;
    border: 1px solid var(--border-default);
    color: var(--fg-primary);
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 6px 12px;
    cursor: pointer;
  }
  .pager button:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .empty-state {
    padding: 48px 0;
    text-align: center;
  }
  .empty {
    margin: 0;
    color: var(--fg-muted);
  }
  .inline-clear {
    background: none;
    border: none;
    color: var(--accent);
    text-decoration: underline;
    font-family: inherit;
    font-size: inherit;
    cursor: pointer;
    padding: 0;
  }
  @media (max-width: 1023px) {
    .layout {
      grid-template-columns: 1fr;
      gap: 24px;
    }
    .sidebar {
      position: static;
    }
    .catalog {
      padding: 24px 16px;
    }
  }
</style>
