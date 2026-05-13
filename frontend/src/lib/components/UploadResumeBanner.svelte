<script lang="ts">
  import { goto, invalidateAll } from '$app/navigation';
  import type { DraftListItem } from '$lib/api/DraftListItem';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';

  interface Props {
    drafts: DraftListItem[];
  }
  let { drafts }: Props = $props();

  // Hide the banner instantly on Discard — invalidateAll() takes a few hundred
  // milliseconds and the user shouldn't see the stale count after confirming.
  let dismissed = $state(false);

  let oldest = $derived(drafts[drafts.length - 1]);
  let relTime = $derived(formatRelative(oldest?.created_at));

  function formatRelative(iso?: string): string {
    if (!iso) return '';
    const ts = Date.parse(iso);
    const diff = Date.now() - ts;
    const minutes = Math.round(diff / 60_000);
    if (minutes < 60) return `${minutes} min ago`;
    const hours = Math.round(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    return new Date(ts).toLocaleDateString();
  }

  function resume() {
    const ids = drafts.map((d) => d.id).join(',');
    goto(`/upload/batch/edit?ids=${ids}`);
  }

  let discardOpen = $state(false);

  async function performDiscardAll() {
    discardOpen = false;
    dismissed = true;
    await Promise.all(
      drafts.map((d) => fetch(`/api/photos/${d.id}`, { method: 'DELETE', credentials: 'include' }))
    );
    await invalidateAll();
  }
</script>

{#if !dismissed}
  <div class="banner">
    <div class="banner-eyebrow">
      ● {drafts.length} DRAFT{drafts.length > 1 ? 'S' : ''} IN PROGRESS
    </div>
    <p class="banner-body">Continue verifying frames from {relTime}.</p>
    <div class="banner-actions">
      <button type="button" class="btn-ghost" onclick={() => (discardOpen = true)}>Discard</button>
      <button type="button" class="btn-primary" onclick={resume}>Resume</button>
    </div>
  </div>
{/if}

<ConfirmDialog
  bind:open={discardOpen}
  title="Discard drafts"
  message={`Discard ${drafts.length} draft${drafts.length === 1 ? '' : 's'}? This cannot be undone.`}
  confirmLabel="Discard"
  tone="danger"
  onconfirm={performDiscardAll}
/>

<style>
  .banner {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 16px;
    padding: 16px 20px;
    border: 1px solid var(--accent);
    background: color-mix(in oklab, var(--accent) 7%, transparent);
    margin-bottom: 32px;
  }
  .banner-eyebrow {
    grid-column: 1;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--accent);
    letter-spacing: 0.12em;
  }
  .banner-body {
    grid-column: 1;
    margin: 4px 0 0;
    font-size: 14px;
    color: var(--fg-secondary);
  }
  .banner-actions {
    grid-column: 2;
    grid-row: 1 / 3;
    display: flex;
    gap: 8px;
  }
  .btn-ghost,
  .btn-primary {
    padding: 8px 16px;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.08em;
    cursor: pointer;
    border: 1px solid var(--border-default);
    background: transparent;
    color: var(--fg-primary);
  }
  .btn-primary {
    background: var(--accent);
    color: var(--bg-base);
    border-color: var(--accent);
  }
</style>
