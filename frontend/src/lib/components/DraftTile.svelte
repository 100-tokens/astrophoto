<script lang="ts">
  import { goto, invalidateAll } from '$app/navigation';
  import { api } from '$lib/api/client';
  import type { DraftListItem } from '$lib/api/DraftListItem';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';

  interface Props {
    draft: DraftListItem;
  }
  let { draft }: Props = $props();

  function relTime(iso: string): string {
    const diff = Date.now() - Date.parse(iso);
    const mins = Math.round(diff / 60_000);
    if (mins < 60) return `${mins} min ago`;
    const hours = Math.round(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    return new Date(iso).toLocaleDateString();
  }

  function statusPip(status: string): string {
    if (status === 'processing') return '⟳ processing';
    if (status === 'failed') return '✗ failed';
    return '✓ ready';
  }

  function resume() {
    goto(`/upload/${draft.id}/verify`);
  }

  let discardOpen = $state(false);

  async function performDiscard() {
    await api.photos.delete(draft.id);
    discardOpen = false;
    await invalidateAll();
  }
</script>

<article class="tile">
  <div class="thumb"><img src={draft.thumb_url} alt={draft.original_name} /></div>
  <div class="meta">
    <p class="title">{draft.target ?? 'untitled'}</p>
    <p class="t-meta">uploaded {relTime(draft.created_at)}</p>
    <p class="t-meta status" data-state={draft.status}>{statusPip(draft.status)}</p>
  </div>
  <div class="actions">
    <button type="button" class="btn-primary" onclick={resume}>Resume</button>
    <button type="button" class="btn-ghost" onclick={() => (discardOpen = true)}>Discard</button>
  </div>
</article>

<ConfirmDialog
  bind:open={discardOpen}
  title="Discard draft"
  message="Discard this draft? This cannot be undone."
  confirmLabel="Discard"
  tone="danger"
  onconfirm={performDiscard}
/>

<style>
  .tile {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    border: 1px solid var(--border-subtle);
  }
  .thumb {
    aspect-ratio: 4 / 3;
    background: var(--bg-elevated);
    overflow: hidden;
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .title {
    font-family: var(--font-display);
    font-size: 15px;
    font-style: italic;
    margin: 0;
  }
  .status[data-state='failed'] {
    color: var(--danger);
  }
  .status[data-state='processing'] {
    color: var(--accent);
  }
  .actions {
    display: flex;
    gap: 8px;
    margin-top: 8px;
  }
  .btn-primary,
  .btn-ghost {
    padding: 6px 12px;
    font-family: var(--font-mono);
    font-size: 11px;
    cursor: pointer;
    border: 1px solid var(--border-default);
    background: transparent;
  }
  .btn-primary {
    background: var(--accent);
    color: var(--bg-base);
    border-color: var(--accent);
  }
</style>
