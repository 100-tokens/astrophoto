<script lang="ts">
  import { goto } from '$app/navigation';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import DiscoveryHeader from '$lib/components/discovery/DiscoveryHeader.svelte';
  import FilterPills from '$lib/components/discovery/FilterPills.svelte';
  import CrossAuthorGrid from '$lib/components/discovery/CrossAuthorGrid.svelte';
  import { fetchTagPage } from '$lib/api/discoveryClient';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

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
    const result = await fetchTagPage(fetch, data.initial.tag.slug, {
      sort: data.sort,
      since: data.since,
      ...(data.category !== undefined ? { category: data.category } : {}),
      cursor: cur,
      limit: 24
    });
    cursor = result.page.next_cursor;
    return { photos: result.page.photos, next_cursor: result.page.next_cursor };
  }
</script>

<AppHeader />
<DiscoveryHeader variant="tag" meta={data.initial.tag} />
<FilterPills
  variant="tag"
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
<AppFooter />
