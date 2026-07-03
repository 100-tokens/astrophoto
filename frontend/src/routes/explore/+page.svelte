<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { untrack } from 'svelte';
  import { cdn } from '$lib/cdn';
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

  // Guards the stale-cursor race: a load-more issued under filter set A
  // must not write its next_cursor after the user switched to filter set
  // B (the write would make the next load-more paginate B's feed from
  // A's position, silently skipping rows).
  let filterKey = $derived(`${data.sort}|${data.since}|${data.category ?? ''}|${data.following}`);

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
    const issuedUnder = filterKey;
    const page = await fetchExplore(fetch, {
      sort: data.sort,
      since: data.since,
      ...(data.category !== undefined ? { category: data.category } : {}),
      following: data.following,
      cursor: cur,
      limit: 24
    });
    if (issuedUnder !== filterKey) {
      // Filters changed while this page was in flight: the {#key} block
      // already remounted the grid and the $effect reseeded `cursor` —
      // don't clobber it with the previous filter's position.
      return { photos: [], next_cursor: cursor };
    }
    cursor = page.next_cursor;
    return { photos: page.photos, next_cursor: page.next_cursor };
  }

  // FilterPills category prop: must not pass undefined with exactOptionalPropertyTypes.
  let pillCategory = $derived(data.category as string | undefined);

  let pageTitle = $derived(
    data.category
      ? `Explore · ${categoryLabel(data.category)} — Astrophoto`
      : 'Explore — Astrophoto'
  );
  const pageDescription =
    'Browse community astrophotography on Astrophoto — filter by category, time window, or photographers you follow.';

  // Canonical normalizes away filter params — sort/since/following views
  // point at the one indexable /explore URL. Category variants instead
  // canonicalize to their dedicated /c/<cat> page: they carry a
  // category-specific <title>, and a canonical that claims sameness while
  // the title differs gets ignored by crawlers (leaving two competing
  // URLs for content whose real home is /c/<cat>).
  let canonicalUrl = $derived(
    data.category ? `${page.url.origin}/c/${data.category}` : `${page.url.origin}/explore`
  );
  let ogImage = $derived.by(() => {
    const first = data.initial.photos[0];
    if (!first) return null;
    const u = cdn(first.id, { w: 1200 });
    return u.startsWith('http') ? u : `${page.url.origin}${u}`;
  });

  function clearFilters() {
    void goto('/explore', { replaceState: true, keepFocus: true, noScroll: true });
  }
</script>

<svelte:head>
  <title>{pageTitle}</title>
  <meta name="description" content={pageDescription} />
  <link rel="canonical" href={canonicalUrl} />

  <meta property="og:type" content="website" />
  <meta property="og:site_name" content="Astrophoto" />
  <meta property="og:title" content={pageTitle} />
  <meta property="og:description" content={pageDescription} />
  <meta property="og:url" content={canonicalUrl} />
  {#if ogImage}
    <meta property="og:image" content={ogImage} />
  {/if}

  <meta name="twitter:card" content={ogImage ? 'summary_large_image' : 'summary'} />
  <meta name="twitter:title" content={pageTitle} />
  <meta name="twitter:description" content={pageDescription} />
  {#if ogImage}
    <meta name="twitter:image" content={ogImage} />
  {/if}
</svelte:head>

<AppHeader active="Gallery" />

<main>
  <!-- No count is honest degradation when /api/site/stats is down — the
       first-page length used to masquerade as the site-wide total. -->
  <DiscoveryHeader
    variant="explore"
    {...data.totalFrames !== null ? { photoCount: data.totalFrames } : {}}
  />
  <FilterPills
    variant="explore"
    sort={data.sort}
    since={data.since}
    {...pillCategory !== undefined ? { category: pillCategory } : {}}
    following={data.following}
    authed={!!data.user}
    onSortChange={(s) => applyFilter({ sort: s })}
    onSinceChange={(s) => applyFilter({ since: s })}
    onCategoryChange={(c) => applyFilter({ category: c })}
    onFollowingChange={(f) => applyFilter({ following: f })}
    onClear={clearFilters}
  />
  {#key `${data.sort}|${data.since}|${data.category ?? ''}|${data.following}`}
    <CrossAuthorGrid
      initial={{ photos: data.initial.photos, next_cursor: data.initial.next_cursor }}
      loadMore={loadMoreFn}
    />
  {/key}
</main>

<LightboxHost />
<AppFooter />
