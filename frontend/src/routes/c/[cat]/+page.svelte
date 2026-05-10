<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import DiscoveryHeader from '$lib/components/discovery/DiscoveryHeader.svelte';
  import FilterPills from '$lib/components/discovery/FilterPills.svelte';
  import CrossAuthorGrid from '$lib/components/discovery/CrossAuthorGrid.svelte';
  import LightboxHost from '$lib/components/discovery/LightboxHost.svelte';
  import { fetchCategoryPage } from '$lib/api/discoveryClient';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  // ── SEO meta ─────────────────────────────────────────────────
  const cat = data.initial.category;
  const catLabel = cat.charAt(0).toUpperCase() + cat.slice(1).toLowerCase();
  const catTitle = `${catLabel} — Astrophoto`;
  const catDescription = `Browse ${catLabel.toLowerCase()} astrophotography on Astrophoto — frames from amateur astrophotographers worldwide, with target catalogue ids and full capture metadata.`;
  const catCanonical = `${page.url.origin}/c/${encodeURIComponent(cat)}`;

  let cursor = $state<string | null>(data.initial.page.next_cursor);
  $effect(() => {
    cursor = data.initial.page.next_cursor;
  });

  function applyFilter(next: { sort?: string; since?: string }) {
    const u = new URL(window.location.href);
    if (next.sort !== undefined) u.searchParams.set('sort', next.sort);
    if (next.since !== undefined) u.searchParams.set('since', next.since);
    void goto(u.pathname + u.search, { replaceState: true, keepFocus: true, noScroll: true });
  }

  async function loadMoreFn() {
    if (!cursor) return { photos: [], next_cursor: null };
    const result = await fetchCategoryPage(fetch, data.initial.category, {
      sort: data.sort,
      since: data.since,
      cursor,
      limit: 24
    });
    cursor = result.page.next_cursor;
    return { photos: result.page.photos, next_cursor: result.page.next_cursor };
  }
</script>

<AppHeader />
<svelte:head>
  <title>{catTitle}</title>
  <meta name="description" content={catDescription} />
  <link rel="canonical" href={catCanonical} />
  <meta property="og:type" content="website" />
  <meta property="og:site_name" content="Astrophoto" />
  <meta property="og:title" content={catTitle} />
  <meta property="og:description" content={catDescription} />
  <meta property="og:url" content={catCanonical} />
  <meta name="twitter:card" content="summary" />
  <meta name="twitter:title" content={catTitle} />
  <meta name="twitter:description" content={catDescription} />
</svelte:head>

<DiscoveryHeader
  variant="category"
  category={data.initial.category}
  photoCount={data.initial.photo_count}
/>
<FilterPills
  variant="category"
  sort={data.sort}
  since={data.since}
  onSortChange={(s) => applyFilter({ sort: s })}
  onSinceChange={(s) => applyFilter({ since: s })}
/>
{#key `${data.sort}|${data.since}`}
  <CrossAuthorGrid
    initial={{ photos: data.initial.page.photos, next_cursor: data.initial.page.next_cursor }}
    loadMore={loadMoreFn}
  />
{/key}
<LightboxHost />
<AppFooter />
