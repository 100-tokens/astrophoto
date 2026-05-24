<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import DiscoveryHeader from '$lib/components/discovery/DiscoveryHeader.svelte';
  import CrossAuthorGrid from '$lib/components/discovery/CrossAuthorGrid.svelte';
  import LightboxHost from '$lib/components/discovery/LightboxHost.svelte';
  import { pluralize } from '$lib/util/pluralize';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  let totalCount = $derived(
    data.initial
      ? data.initial.targets.length + data.initial.users.length + data.initial.photos.length
      : 0
  );

  // ── SEO meta ─────────────────────────────────────────────────
  let pageTitle = $derived(
    data.q.trim() ? `Search · ${data.q} — Astrophoto` : 'Search — Astrophoto'
  );
  let pageDescription = $derived(
    data.q.trim()
      ? `${totalCount} result${totalCount === 1 ? '' : 's'} for "${data.q}" on Astrophoto — targets, photographers, photos.`
      : 'Search Astrophoto — find targets (M, NGC, IC, Caldwell), photographers, and photos.'
  );
</script>

<svelte:head>
  <title>{pageTitle}</title>
  <meta name="description" content={pageDescription} />
  <!-- robots.txt allows /search; noindex per-page so we don't pollute
       the search engine index with infinite query permutations. -->
  <meta name="robots" content="noindex, follow" />
</svelte:head>

<AppHeader />

<main>
  {#if data.q.trim()}
    <DiscoveryHeader variant="search" q={data.q} resultCount={totalCount} />
  {:else}
    <section class="empty-state">
      <p class="empty-label">Enter a search term to find targets, photographers, and photos.</p>
    </section>
  {/if}

  {#if data.initial}
    {#if data.initial.targets.length > 0}
      <section class="results-section">
        <h2 class="section-heading">Targets</h2>
        <ul class="result-list">
          {#each data.initial.targets as target (target.slug)}
            <li>
              <a href="/t/{target.slug}" class="result-link">
                <span class="result-primary">{target.canonical_name}</span>
                <span class="result-meta">{pluralize(target.photo_count, 'photo')}</span>
              </a>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if data.initial.users.length > 0}
      <section class="results-section">
        <h2 class="section-heading">Photographers</h2>
        <ul class="result-list">
          {#each data.initial.users as user (user.id)}
            <li>
              <a href="/u/{user.handle}" class="result-link">
                <span class="result-primary">{user.display_name}</span>
                <span class="result-meta">@{user.handle}</span>
              </a>
            </li>
          {/each}
        </ul>
      </section>
    {/if}

    {#if data.initial.photos.length > 0}
      <section class="results-section">
        <h2 class="section-heading">Photos</h2>
        <CrossAuthorGrid initial={{ photos: data.initial.photos, next_cursor: null }} />
      </section>
    {/if}

    {#if totalCount === 0}
      <p class="no-results">No results found for "{data.q}".</p>
    {/if}
  {/if}
</main>

<LightboxHost />
<AppFooter />

<style>
  .empty-state {
    padding: 48px 64px;
  }

  .empty-label {
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--fg-muted);
    margin: 0;
  }

  .results-section {
    padding: 32px 0 0 0;
    border-top: 1px solid var(--border-subtle);
    margin-top: 8px;
  }

  .results-section:first-of-type {
    border-top: none;
  }

  .section-heading {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.08em;
    color: var(--fg-muted);
    text-transform: uppercase;
    margin: 0 0 16px 64px;
  }

  .result-list {
    list-style: none;
    margin: 0;
    padding: 0 64px;
  }

  .result-link {
    display: flex;
    align-items: baseline;
    gap: 12px;
    padding: 10px 0;
    border-bottom: 1px solid var(--border-subtle);
    text-decoration: none;
    color: var(--fg-primary);
    transition: color 0.1s;
  }

  .result-link:hover {
    color: var(--accent);
  }

  .result-primary {
    font-size: 15px;
    font-weight: 500;
  }

  .result-meta {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
  }

  .no-results {
    padding: 48px 64px;
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--fg-muted);
  }
</style>
