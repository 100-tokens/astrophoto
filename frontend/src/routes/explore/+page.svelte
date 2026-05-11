<script lang="ts">
  import { goto } from '$app/navigation';
  import { untrack } from 'svelte';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import DiscoveryHeader from '$lib/components/discovery/DiscoveryHeader.svelte';
  import FilterPills from '$lib/components/discovery/FilterPills.svelte';
  import CrossAuthorGrid from '$lib/components/discovery/CrossAuthorGrid.svelte';
  import LightboxHost from '$lib/components/discovery/LightboxHost.svelte';
  import { fetchExplore } from '$lib/api/discoveryClient';
  import { categoryLabel } from '$lib/util/categoryLabel';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  // Track cursor for load-more closed over current filter state. Seed once
  // from the SSR data; the $effect below resets it on filter navigation.
  let cursor = $state<string | null>(untrack(() => data.initial.next_cursor));
  // Reset when data changes (navigation with new filter params).
  $effect(() => {
    cursor = data.initial.next_cursor;
  });

  function applyFilter(next: {
    sort?: string;
    since?: string;
    category?: string | undefined;
    following?: boolean;
  }) {
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
    if (next.following !== undefined) {
      if (next.following) {
        u.searchParams.set('following', 'true');
      } else {
        u.searchParams.delete('following');
      }
    }
    void goto(u.pathname + u.search, { replaceState: true, keepFocus: true, noScroll: true });
  }

  async function loadMoreFn() {
    if (!cursor) return { photos: [], next_cursor: null };
    const cur = cursor;
    const page = await fetchExplore(fetch, {
      sort: data.sort,
      since: data.since,
      ...(data.category !== undefined ? { category: data.category } : {}),
      following: data.following,
      cursor: cur,
      limit: 24
    });
    cursor = page.next_cursor;
    return { photos: page.photos, next_cursor: page.next_cursor };
  }

  // FilterPills category prop: must not pass undefined with exactOptionalPropertyTypes.
  let pillCategory = $derived(data.category as string | undefined);

  let pageTitle = $derived(
    data.category ? `Explore · ${categoryLabel(data.category)} — Astrophoto` : 'Explore — Astrophoto'
  );
  const pageDescription =
    'Browse community astrophotography on Astrophoto — filter by category, time window, or photographers you follow.';
</script>

<svelte:head>
  <title>{pageTitle}</title>
  <meta name="description" content={pageDescription} />
</svelte:head>

<AppHeader />
<DiscoveryHeader variant="explore" photoCount={data.initial.photos.length} />
<FilterPills
  variant="explore"
  sort={data.sort}
  since={data.since}
  {...pillCategory !== undefined ? { category: pillCategory } : {}}
  following={data.following}
  onSortChange={(s) => applyFilter({ sort: s })}
  onSinceChange={(s) => applyFilter({ since: s })}
  onCategoryChange={(c) => applyFilter({ category: c })}
  onFollowingChange={(f) => applyFilter({ following: f })}
/>
{#key `${data.sort}|${data.since}|${data.category ?? ''}|${data.following}`}
  <CrossAuthorGrid
    initial={{ photos: data.initial.photos, next_cursor: data.initial.next_cursor }}
    loadMore={loadMoreFn}
  />
{/key}
<LightboxHost />
<AppFooter />
