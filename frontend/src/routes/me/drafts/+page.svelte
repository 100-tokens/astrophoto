<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import DraftTile from '$lib/components/DraftTile.svelte';
  import Button from '$lib/components/Button.svelte';
  import { goto } from '$app/navigation';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();

  function resumeRecent() {
    if (data.drafts.items.length === 0) return;
    const newest = Date.parse(data.drafts.items[0]!.created_at);
    const cutoff = newest - 60 * 60 * 1000;
    const ids = data.drafts.items
      .filter((d) => Date.parse(d.created_at) >= cutoff)
      .map((d) => d.id);
    if (ids.length === 1) goto(`/upload/${ids[0]}/verify`);
    else goto(`/upload/batch/edit?ids=${ids.join(',')}`);
  }
</script>

<svelte:head><title>Drafts — Astrophoto</title></svelte:head>
<AppHeader active="Gallery" />

<div class="page">
  <div class="header">
    <div>
      <div class="t-eyebrow">DRAFTS</div>
      <h1 class="title">Drafts <span class="count">· {data.drafts.items.length}</span></h1>
    </div>
    {#if data.drafts.items.length > 0}
      <Button variant="primary" type="button" onclick={resumeRecent}>Resume recent</Button>
    {/if}
  </div>

  {#if data.drafts.items.length === 0}
    <p class="empty">No drafts. <a href="/upload">Upload a frame →</a></p>
  {:else}
    <div class="grid">
      {#each data.drafts.items as draft (draft.id)}
        <DraftTile {draft} />
      {/each}
    </div>
    {#if data.drafts.next_cursor}
      <div class="pager">
        <Button variant="ghost" href={`/me/drafts?cursor=${data.drafts.next_cursor}`}>Older →</Button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .page { max-width: 1200px; margin: 0 auto; padding: 40px 64px 64px; }
  .header { display: flex; justify-content: space-between; align-items: end; margin-bottom: 32px; }
  .title { font-family: var(--font-display); font-size: 44px; margin: 8px 0 0; }
  .count { color: var(--fg-muted); font-size: 28px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 16px; }
  .empty { color: var(--fg-secondary); }
  .pager { display: flex; justify-content: center; margin-top: 32px; }
  @media (max-width: 768px) { .page { padding: 32px 24px; } }
</style>
