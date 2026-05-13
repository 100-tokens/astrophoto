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
  import AladinSkyMap from '$lib/components/discovery/AladinSkyMap.svelte';
  import ExternalArchiveLinks from '$lib/components/discovery/ExternalArchiveLinks.svelte';
  import { fetchTargetPage } from '$lib/api/discoveryClient';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  // ── SEO / GEO meta ─────────────────────────────────────────────
  let target = $derived(data.initial.target);
  let photoCount = $derived(Number(target.photo_count));
  let contributorCount = $derived(Number(target.contributor_count));
  let targetTitle = $derived(`${target.canonical_name} — Astrophoto`);
  let targetDescription = $derived(
    `${photoCount} frame${photoCount === 1 ? '' : 's'} of ${target.canonical_name}${target.constellation ? ` in ${target.constellation}` : ''}, captured by ${contributorCount} astrophotographer${contributorCount === 1 ? '' : 's'} on Astrophoto.`
  );
  let targetCanonical = $derived(`${page.url.origin}/t/${encodeURIComponent(target.slug)}`);
  let targetJsonLd = $derived(
    JSON.stringify({
      '@context': 'https://schema.org',
      '@type': 'CollectionPage',
      '@id': targetCanonical,
      name: target.canonical_name,
      description: targetDescription,
      url: targetCanonical,
      about: {
        '@type': 'Thing',
        name: target.canonical_name,
        ...(target.aliases?.length ? { alternateName: target.aliases } : {}),
        ...(target.constellation ? { containedInPlace: target.constellation } : {})
      },
      numberOfItems: photoCount
    }).replace(/</g, '\\u003c')
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
    const result = await fetchTargetPage(fetch, data.initial.target.slug, {
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

<AppHeader active="Targets" />
<svelte:head>
  <title>{targetTitle}</title>
  <meta name="description" content={targetDescription} />
  <link rel="canonical" href={targetCanonical} />
  <meta property="og:type" content="website" />
  <meta property="og:site_name" content="Astrophoto" />
  <meta property="og:title" content={targetTitle} />
  <meta property="og:description" content={targetDescription} />
  <meta property="og:url" content={targetCanonical} />
  <meta name="twitter:card" content="summary" />
  <meta name="twitter:title" content={targetTitle} />
  <meta name="twitter:description" content={targetDescription} />
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html `<script type="application/ld+json">${targetJsonLd}</script>`}
</svelte:head>

<DiscoveryHeader variant="target" meta={data.initial.target} />
<AladinSkyMap
  ra={data.initial.target.right_ascension}
  dec={data.initial.target.declination}
  majorAxisArcmin={data.initial.target.major_axis_arcmin}
  objectName={data.initial.target.canonical_name}
/>
<FilterPills
  variant="target"
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
<ExternalArchiveLinks
  canonicalName={data.initial.target.canonical_name}
  aliases={data.initial.target.aliases}
  slug={data.initial.target.slug}
/>
<LightboxHost />
<AppFooter />
