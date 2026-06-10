<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { untrack } from 'svelte';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import DiscoveryHeader from '$lib/components/discovery/DiscoveryHeader.svelte';
  import FilterPills from '$lib/components/discovery/FilterPills.svelte';
  import CrossAuthorGrid from '$lib/components/discovery/CrossAuthorGrid.svelte';
  import LightboxHost from '$lib/components/discovery/LightboxHost.svelte';
  import { fetchCategoryPage } from '$lib/api/discoveryClient';
  import { categoryLabel } from '$lib/util/categoryLabel';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  // ── SEO meta ─────────────────────────────────────────────────
  // Derived (not const) so navigating between categories updates the head
  // without remounting the page. page.url.origin is reactive too.
  let cat = $derived(data.initial.category);
  let catLabel = $derived(categoryLabel(cat));
  let catTitle = $derived(`${catLabel} — Astrophoto`);
  let catDescription = $derived(
    `Browse ${catLabel.toLowerCase()} astrophotography on Astrophoto — frames from amateur astrophotographers worldwide, with target catalogue ids and full capture metadata.`
  );
  let catCanonical = $derived(`${page.url.origin}/c/${encodeURIComponent(cat)}`);

  let cursor = $state<string | null>(untrack(() => data.initial.page.next_cursor));
  $effect(() => {
    cursor = data.initial.page.next_cursor;
  });

  function applyFilter(next: { sort?: string }) {
    const u = new URL(window.location.href);
    if (next.sort !== undefined) u.searchParams.set('sort', next.sort);
    void goto(u.pathname + u.search, { replaceState: true, keepFocus: true, noScroll: true });
  }

  async function loadMoreFn() {
    if (!cursor) return { photos: [], next_cursor: null };
    const result = await fetchCategoryPage(fetch, data.initial.category, {
      sort: data.sort,
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

<main>
  <DiscoveryHeader
    variant="category"
    category={data.initial.category}
    photoCount={data.initial.photo_count}
  />
  <!-- No `since` here: only /api/explore implements it; the category endpoint
       silently ignores the param, so forwarding it would lie to the user. -->
  <FilterPills variant="category" sort={data.sort} onSortChange={(s) => applyFilter({ sort: s })} />
  <!-- The category must be part of the key: same-route navigation between two
       categories reuses this page instance, and CrossAuthorGrid keeps its own
       loaded-pages state, which would leak across categories. -->
  {#key `${data.initial.category}|${data.sort}`}
    <CrossAuthorGrid
      initial={{ photos: data.initial.page.photos, next_cursor: data.initial.page.next_cursor }}
      loadMore={loadMoreFn}
    />
  {/key}
</main>

<LightboxHost />
<AppFooter />
