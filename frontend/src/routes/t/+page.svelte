<script lang="ts">
  import { goto } from '$app/navigation';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import TargetIndexCard from '$lib/components/discovery/TargetIndexCard.svelte';
  import { fetchTargetList } from '$lib/api/discoveryClient';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  let items = $state(data.initial.targets);
  let cursor = $state(data.initial.next_cursor);
  let q = $state(data.q ?? '');
  let qDebounceTimer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    items = data.initial.targets;
    cursor = data.initial.next_cursor;
    q = data.q ?? '';
  });

  function applyFilter(next: {
    q?: string;
    sort?: string;
    object_type?: string | undefined;
    constellation?: string | undefined;
  }) {
    const u = new URL(window.location.href);
    if (next.q !== undefined) {
      if (next.q) u.searchParams.set('q', next.q);
      else u.searchParams.delete('q');
    }
    if (next.sort !== undefined) u.searchParams.set('sort', next.sort);
    if ('object_type' in next) {
      if (next.object_type) u.searchParams.set('object_type', next.object_type);
      else u.searchParams.delete('object_type');
    }
    if ('constellation' in next) {
      if (next.constellation) u.searchParams.set('constellation', next.constellation);
      else u.searchParams.delete('constellation');
    }
    u.searchParams.delete('cursor');
    void goto(u.pathname + u.search, { replaceState: true, keepFocus: true, noScroll: true });
  }

  function onSearchInput(value: string) {
    q = value;
    if (qDebounceTimer) clearTimeout(qDebounceTimer);
    qDebounceTimer = setTimeout(() => applyFilter({ q }), 200);
  }

  async function loadMore() {
    if (!cursor) return;
    const next = await fetchTargetList(fetch, {
      ...(data.q !== undefined ? { q: data.q } : {}),
      sort: data.sort,
      ...(data.object_type !== undefined ? { object_type: data.object_type } : {}),
      ...(data.constellation !== undefined ? { constellation: data.constellation } : {}),
      cursor,
      limit: 24
    });
    items = [...items, ...next.targets];
    cursor = next.next_cursor;
  }
</script>

<svelte:head>
  <title>Celestial objects — Astrophoto</title>
  <meta
    name="description"
    content="Explore thousands of galaxies, nebulae, and clusters photographed by the community."
  />
</svelte:head>

<AppHeader />

<main class="t-index">
  <header class="header-row">
    <h1>Celestial objects</h1>
    <input
      type="search"
      class="input search-input"
      placeholder="Search for an object…"
      value={q}
      oninput={(e) => onSearchInput((e.target as HTMLInputElement).value)}
    />
  </header>

  <div class="filters">
    <label>
      Type
      <select
        onchange={(e) =>
          applyFilter({ object_type: (e.target as HTMLSelectElement).value || undefined })}
      >
        <option value="" selected={!data.object_type}>All</option>
        <option value="G" selected={data.object_type === 'G'}>Galaxy</option>
        <option value="Neb" selected={data.object_type === 'Neb'}>Nebula</option>
        <option value="OCl" selected={data.object_type === 'OCl'}>Open cluster</option>
        <option value="GCl" selected={data.object_type === 'GCl'}>Globular cluster</option>
        <option value="PN" selected={data.object_type === 'PN'}>Planetary nebula</option>
        <option value="HII" selected={data.object_type === 'HII'}>HII region</option>
        <option value="SNR" selected={data.object_type === 'SNR'}>Supernova remnant</option>
      </select>
    </label>
    <label>
      Sort
      <select onchange={(e) => applyFilter({ sort: (e.target as HTMLSelectElement).value })}>
        <option value="popular" selected={data.sort === 'popular'}>Popular</option>
        <option value="name" selected={data.sort === 'name'}>Alphabetical</option>
      </select>
    </label>
  </div>

  {#if items.length === 0}
    <p class="empty">
      No objects match.
      <button
        type="button"
        onclick={() => applyFilter({ q: '', object_type: undefined, constellation: undefined })}
      >
        Clear filters
      </button>
    </p>
  {:else}
    <ul class="grid">
      {#each items as t (t.slug)}
        <li><TargetIndexCard target={t} /></li>
      {/each}
    </ul>
    {#if cursor}
      <button type="button" class="load-more" onclick={() => void loadMore()}>
        Load more
      </button>
    {/if}
  {/if}

  <footer class="data-attrib">
    Catalog data:
    <a href="https://github.com/mattiaverga/OpenNGC">OpenNGC by Mattia Verga and contributors</a>
    —
    <a href="https://creativecommons.org/licenses/by-sa/4.0/">CC-BY-SA 4.0</a>. Adapted to slug
    format and merged with manual catalog seed.
  </footer>
</main>

<AppFooter />

<style>
  .t-index {
    max-width: 1200px;
    margin: 0 auto;
    padding: 1.5rem 1rem;
  }
  .header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    margin-bottom: 1rem;
  }
  .header-row h1 {
    margin: 0;
  }
  .search-input {
    flex: 0 1 320px;
  }
  .filters {
    display: flex;
    gap: 1rem;
    margin-bottom: 1rem;
    font-size: 0.9rem;
  }
  .filters label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    color: var(--fg-muted, #888);
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .filters select {
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 4px 8px;
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 1rem;
  }
  .grid > li {
    list-style: none;
  }
  .empty {
    padding: 2rem;
    text-align: center;
    color: var(--fg-muted, #888);
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .empty button {
    background: transparent;
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 4px 12px;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
    margin-left: 0.5rem;
  }
  .load-more {
    display: block;
    margin: 2rem auto;
    padding: 8px 16px;
    background: transparent;
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
  .load-more:hover {
    border-color: var(--accent, #4a90e2);
  }
  .data-attrib {
    margin-top: 3rem;
    padding-top: 1rem;
    border-top: 1px solid var(--border-subtle, #ddd);
    font-size: 0.75rem;
    color: var(--fg-muted, #888);
    text-align: center;
    font-family: var(--font-mono);
  }
  .data-attrib a {
    color: inherit;
    text-decoration: underline;
  }
</style>
