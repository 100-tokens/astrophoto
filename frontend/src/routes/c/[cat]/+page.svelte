<script lang="ts">
  import { goto } from '$app/navigation';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import DiscoveryHeader from '$lib/components/discovery/DiscoveryHeader.svelte';
  import FilterPills from '$lib/components/discovery/FilterPills.svelte';
  import CrossAuthorGrid from '$lib/components/discovery/CrossAuthorGrid.svelte';
  import { fetchCategoryPage } from '$lib/api/discoveryClient';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

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
<AppFooter />
