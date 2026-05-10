<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import DiscoveryHeader from '$lib/components/discovery/DiscoveryHeader.svelte';
  import FilterPills from '$lib/components/discovery/FilterPills.svelte';
  import CrossAuthorGrid from '$lib/components/discovery/CrossAuthorGrid.svelte';
  import EquipmentPairedRail from '$lib/components/discovery/EquipmentPairedRail.svelte';
  import LightboxHost from '$lib/components/discovery/LightboxHost.svelte';
  import { fetchEquipmentPage } from '$lib/api/discoveryClient';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  // ── SEO meta ─────────────────────────────────────────────────
  const eq = data.initial.equipment;
  const eqPhotos = Number(eq.photo_count);
  const eqLabel = `${eq.display_name} (${eq.kind})`;
  const eqTitle = `${eqLabel} — Astrophoto`;
  const eqDescription = `${eqPhotos} astrophotograph${eqPhotos === 1 ? '' : 's'} captured with a ${eq.display_name} on Astrophoto. See what amateur astrophotographers shoot with this ${eq.kind}.`;
  const eqCanonical = `${page.url.origin}/equip/${encodeURIComponent(eq.kind)}/${encodeURIComponent(eq.slug)}`;

  let cursor = $state<string | null>(data.initial.page.next_cursor);
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
<EquipmentPairedRail items={data.initial.paired} />
<LightboxHost />
<AppFooter />
