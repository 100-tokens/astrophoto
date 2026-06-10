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
  import { fetchTagPage } from '$lib/api/discoveryClient';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  // ── SEO meta ─────────────────────────────────────────────────
  let tag = $derived(data.initial.tag);
  let tagPhotos = $derived(Number(tag.photo_count));
  let tagTitle = $derived(`#${tag.name} — Astrophoto`);
  let tagDescription = $derived(
    `${tagPhotos} astrophotograph${tagPhotos === 1 ? '' : 's'} tagged #${tag.name} on Astrophoto.`
  );
  let tagCanonical = $derived(`${page.url.origin}/tag/${encodeURIComponent(tag.slug)}`);

  let cursor = $state<string | null>(untrack(() => data.initial.page.next_cursor));
  $effect(() => {
    cursor = data.initial.page.next_cursor;
  });

  function applyFilter(next: { sort?: string; category?: string | undefined }) {
    const u = new URL(window.location.href);
    if (next.sort !== undefined) u.searchParams.set('sort', next.sort);
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
      ...(data.category !== undefined ? { category: data.category } : {}),
      cursor: cur,
      limit: 24
    });
    cursor = result.page.next_cursor;
    return { photos: result.page.photos, next_cursor: result.page.next_cursor };
  }
</script>

<AppHeader />

<svelte:head>
  <title>{tagTitle}</title>
  <meta name="description" content={tagDescription} />
  <link rel="canonical" href={tagCanonical} />
  <meta property="og:type" content="website" />
  <meta property="og:site_name" content="Astrophoto" />
  <meta property="og:title" content={tagTitle} />
  <meta property="og:description" content={tagDescription} />
  <meta property="og:url" content={tagCanonical} />
  <meta name="twitter:card" content="summary" />
  <meta name="twitter:title" content={tagTitle} />
  <meta name="twitter:description" content={tagDescription} />
</svelte:head>

<main>
  <DiscoveryHeader variant="tag" meta={data.initial.tag} />
  <!-- No `since` here: only /api/explore implements it; the tag endpoint
       silently ignores the param, so forwarding it would lie to the user. -->
  <FilterPills
    variant="tag"
    sort={data.sort}
    {...data.category !== undefined ? { category: data.category } : {}}
    onSortChange={(s) => applyFilter({ sort: s })}
    onCategoryChange={(c) => applyFilter({ category: c })}
  />
  <!-- The slug must be part of the key: same-route navigation between two
       tags reuses this page instance, and CrossAuthorGrid keeps its own
       loaded-pages state, which would leak across tags. -->
  {#key `${data.initial.tag.slug}|${data.sort}|${data.category ?? ''}`}
    <CrossAuthorGrid
      initial={{ photos: data.initial.page.photos, next_cursor: data.initial.page.next_cursor }}
      loadMore={loadMoreFn}
    />
  {/key}
</main>

<LightboxHost />
<AppFooter />
