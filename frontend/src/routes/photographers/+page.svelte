<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { untrack } from 'svelte';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import Img from '$lib/components/Img.svelte';
  import type { PageProps } from './$types';
  import type { PhotographerListItem } from '$lib/api/PhotographerListItem';

  let { data }: PageProps = $props();

  let items = $state<PhotographerListItem[]>(untrack(() => data.initial.items));
  let cursor = $state<string | null>(untrack(() => data.initial.next_cursor));
  let loading = $state(false);
  let loadError = $state<string | null>(null);

  $effect(() => {
    items = data.initial.items;
    cursor = data.initial.next_cursor;
  });

  function pickSort(next: 'active' | 'followers' | 'recent') {
    const u = new URL(window.location.href);
    u.searchParams.set('sort', next);
    void goto(u.pathname + u.search, { keepFocus: true, noScroll: true });
  }

  async function loadMore() {
    if (!cursor || loading) return;
    loading = true;
    loadError = null;
    try {
      const r = await fetch(
        `/api/photographers?sort=${data.sort}&cursor=${encodeURIComponent(cursor)}&limit=24`
      );
      if (!r.ok) throw new Error(`backend ${r.status}`);
      const next = await r.json();
      items = [...items, ...next.items];
      cursor = next.next_cursor;
    } catch (e) {
      loadError = (e as Error).message;
    } finally {
      loading = false;
    }
  }

  function formatHours(seconds: number): string {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    if (h === 0 && m === 0) return '—';
    if (h === 0) return `${m}m`;
    return `${h}h`;
  }

  // ── SEO meta ─────────────────────────────────────────────────
  let pageTitle = 'Photographers — Astrophoto';
  let pageDescription =
    'Amateur astrophotographers on Astrophoto, ordered by published frames. Browse the people behind the photos — bios, equipment, locations, archives.';
  let canonical = $derived(`${page.url.origin}/photographers`);

  // ItemList JSON-LD so AI engines can answer "who are the most-active
  // astrophotographers on Astrophoto?" — surfaces the top N as a ranked
  // list pointing back at each /u/<handle> profile.
  let listJsonLd = $derived(
    JSON.stringify({
      '@context': 'https://schema.org',
      '@type': 'CollectionPage',
      '@id': canonical,
      name: 'Photographers',
      url: canonical,
      hasPart: {
        '@type': 'ItemList',
        itemListOrder: 'https://schema.org/ItemListOrderDescending',
        numberOfItems: items.length,
        itemListElement: items.slice(0, 50).map((p, i) => ({
          '@type': 'ListItem',
          position: i + 1,
          url: `${page.url.origin}/u/${encodeURIComponent(p.handle)}`,
          item: {
            '@type': 'Person',
            name: p.display_name,
            alternateName: `@${p.handle}`,
            url: `${page.url.origin}/u/${encodeURIComponent(p.handle)}`
          }
        }))
      }
    }).replace(/</g, '\\u003c')
  );
</script>

<svelte:head>
  <title>{pageTitle}</title>
  <meta name="description" content={pageDescription} />
  <link rel="canonical" href={canonical} />
  <meta property="og:type" content="website" />
  <meta property="og:site_name" content="Astrophoto" />
  <meta property="og:title" content={pageTitle} />
  <meta property="og:description" content={pageDescription} />
  <meta property="og:url" content={canonical} />
  <meta name="twitter:card" content="summary" />
  <meta name="twitter:title" content={pageTitle} />
  <meta name="twitter:description" content={pageDescription} />
  <!-- eslint-disable-next-line svelte/no-at-html-tags -->
  {@html `<script type="application/ld+json">${listJsonLd}</script>`}
</svelte:head>

<AppHeader active="Photographers" />

<section class="page-header">
  <div class="t-eyebrow">PHOTOGRAPHERS · {items.length}</div>
  <h1 class="page-title">The people behind the <em>photos</em></h1>

  <nav class="sort-pills" aria-label="Sort photographers">
    <button class="pill" class:on={data.sort === 'active'} onclick={() => pickSort('active')}
      >Most active</button
    >
    <button
      class="pill"
      class:on={data.sort === 'followers'}
      onclick={() => pickSort('followers')}>Most followed</button
    >
    <button class="pill" class:on={data.sort === 'recent'} onclick={() => pickSort('recent')}
      >Newest</button
    >
  </nav>
</section>

<section class="grid">
  {#each items as p (p.handle)}
    <a class="card" href={`/u/${encodeURIComponent(p.handle)}`} aria-label={p.display_name}>
      <div class="cover">
        {#if p.cover_photo_id}
          <Img photoId={p.cover_photo_id} alt="" w={400} />
        {:else}
          <span class="cover-fallback" aria-hidden="true">
            {p.display_name[0]?.toUpperCase() ?? '·'}
          </span>
        {/if}
      </div>
      <div class="meta">
        <div class="name">{p.display_name}</div>
        <div class="handle t-mono">@{p.handle}</div>
        <div class="stats t-meta">
          <span>{p.frame_count}</span><span>frame{Number(p.frame_count) === 1 ? '' : 's'}</span>
          <span class="sep">·</span>
          <span>{formatHours(Number(p.integration_seconds))}</span><span>integration</span>
          {#if Number(p.follower_count) > 0}
            <span class="sep">·</span>
            <span>{p.follower_count}</span><span>followers</span>
          {/if}
        </div>
      </div>
    </a>
  {/each}
</section>

{#if items.length === 0}
  <p class="empty t-meta">No photographers yet. Be the first to <a href="/upload">publish a frame</a>.</p>
{/if}

{#if cursor}
  <div class="more">
    <button class="btn-ghost" onclick={loadMore} disabled={loading}>
      {loading ? 'Loading…' : 'Load more →'}
    </button>
    {#if loadError}<span class="err">{loadError}</span>{/if}
  </div>
{/if}

<AppFooter />

<style>
  .page-header {
    padding: 40px 64px 24px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .page-title {
    font-family: var(--font-display);
    font-size: 48px;
    font-weight: 400;
    margin: 8px 0 24px;
    line-height: 1;
  }
  .sort-pills {
    display: flex;
    gap: 8px;
    margin-top: 16px;
  }
  .pill {
    background: transparent;
    border: 1px solid var(--border-default);
    color: var(--fg-secondary);
    padding: 8px 14px;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.06em;
    cursor: pointer;
  }
  .pill:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
  .pill.on {
    background: var(--accent);
    color: var(--accent-ink);
    border-color: var(--accent);
  }

  .grid {
    padding: 32px 64px;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 24px;
  }
  .card {
    display: block;
    text-decoration: none;
    color: inherit;
    border: 1px solid var(--border-subtle);
    transition: border-color 0.15s;
  }
  .card:hover {
    border-color: var(--accent);
  }
  .cover {
    aspect-ratio: 4 / 3;
    background: var(--bg-elevated);
    position: relative;
    overflow: hidden;
    display: grid;
    place-items: center;
  }
  .cover :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .cover-fallback {
    font-family: var(--font-display);
    font-size: 64px;
    color: var(--accent);
    font-style: italic;
  }
  .meta {
    padding: 12px 14px 16px;
  }
  .name {
    font-family: var(--font-display);
    font-size: 18px;
    font-style: italic;
    margin: 0;
  }
  .handle {
    font-size: 11px;
    color: var(--fg-muted);
    margin-top: 2px;
  }
  .stats {
    margin-top: 8px;
    color: var(--fg-secondary);
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    font-size: 11px;
  }
  .stats .sep {
    color: var(--fg-faint);
  }

  .more {
    padding: 32px 64px 64px;
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 16px;
  }
  .btn-ghost {
    background: transparent;
    border: 1px solid var(--border-default);
    color: var(--fg-secondary);
    padding: 12px 24px;
    font-family: var(--font-mono);
    font-size: 12px;
    letter-spacing: 0.08em;
    cursor: pointer;
  }
  .btn-ghost:hover:not(:disabled) {
    color: var(--accent);
    border-color: var(--accent);
  }
  .btn-ghost:disabled {
    opacity: 0.6;
    cursor: progress;
  }
  .err {
    color: var(--danger);
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .empty {
    padding: 48px 64px;
    color: var(--fg-muted);
    text-align: center;
  }
  .empty a {
    color: var(--accent);
  }

  @media (max-width: 1024px) {
    .grid {
      grid-template-columns: repeat(3, 1fr);
    }
  }
  @media (max-width: 768px) {
    .grid {
      grid-template-columns: repeat(2, 1fr);
      gap: 16px;
      padding: 24px;
    }
    .page-header {
      padding: 32px 24px 16px;
    }
    .page-title {
      font-size: 32px;
    }
    .more {
      padding: 24px;
    }
  }
</style>
