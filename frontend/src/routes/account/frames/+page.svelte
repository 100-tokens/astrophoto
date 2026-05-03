<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import StatsRow from '$lib/components/photos/StatsRow.svelte';
  import DraftsCallout from '$lib/components/photos/DraftsCallout.svelte';
  import FilterChips from '$lib/components/photos/FilterChips.svelte';
  import PhotosTable from '$lib/components/photos/PhotosTable.svelte';
  import Button from '$lib/components/Button.svelte';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  let isEmpty = $derived(data.counts.all === 0);
</script>

<svelte:head><title>My frames — Astrophoto</title></svelte:head>
<!-- No active prop: no nav link matches /account/frames -->
<AppHeader />

<div class="frames-page">
  {#if isEmpty}
    <div class="empty">
      <h1>An empty plate, waiting for first light.</h1>
      <Button variant="primary" href="/upload" size="lg">Upload a frame</Button>
    </div>
  {:else}
    <header class="title-row">
      <h1>My frames</h1>
      <StatsRow stats={data.stats} />
    </header>

    {#if data.drafts.length > 0 && data.filter !== 'drafts'}
      <DraftsCallout drafts={data.drafts} />
    {/if}

    <FilterChips active={data.filter} counts={data.counts} sort={data.sort} view={data.view} />

    {#if data.filter === 'drafts' && data.drafts.length === 0}
      <p class="empty-msg">
        No drafts. Every frame you upload is published.
        <a href="/upload">Upload a frame</a>
      </p>
    {:else}
      <PhotosTable rows={data.rows} />
    {/if}
  {/if}
</div>

<style>
  .frames-page {
    padding: 40px 64px;
    max-width: 1280px;
    margin: 0 auto;
  }
  .title-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    margin-bottom: 32px;
  }
  .title-row h1 {
    font-family: var(--font-display);
    font-size: 44px;
    margin: 0;
  }
  .empty {
    text-align: center;
    padding: 120px 24px;
  }
  .empty h1 {
    font-family: var(--font-display);
    font-size: 32px;
    margin-bottom: 24px;
  }
  .empty-msg {
    padding: 40px 0;
    color: var(--fg-secondary);
  }
  @media (max-width: 768px) {
    .frames-page {
      padding: 32px 24px;
    }
    .title-row {
      flex-direction: column;
      gap: 16px;
      align-items: flex-start;
    }
  }
</style>
