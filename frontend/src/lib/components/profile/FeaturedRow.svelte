<script lang="ts">
  import type { FeaturedPhotoSummary } from '$lib/api/FeaturedPhotoSummary';
  import { unpinFeatured, reorderFeatured } from '$lib/api/profileClient';
  import FeaturedTile from './FeaturedTile.svelte';
  import FeaturedPinModal from './editor/FeaturedPinModal.svelte';

  let {
    items: incoming,
    handle,
    isOwner,
    editorMode = false,
    onPinned = () => {}
  }: {
    items: FeaturedPhotoSummary[];
    handle: string;
    isOwner: boolean;
    editorMode?: boolean;
    onPinned?: (photoId: string) => void;
  } = $props();

  let local = $state<FeaturedPhotoSummary[]>([...incoming]);
  $effect(() => {
    local = [...incoming];
  });

  let pinOpen = $state(false);
  let placeholders = $derived(isOwner ? Array.from({ length: 6 - local.length }, (_, i) => i) : []);

  async function unpin(id: string) {
    const next = local
      .filter((p) => p.id !== id)
      .map((p, i) => ({ ...p, featured_position: i + 1 }));
    local = next;
    try {
      await unpinFeatured(fetch, id);
    } catch (_e) {
      local = [...incoming];
    }
  }

  async function reorderTo(from: number, to: number) {
    if (from === to || from < 0 || to < 0 || from >= local.length || to >= local.length) return;
    const moved = [...local];
    const it = moved[from];
    if (!it) return;
    moved.splice(from, 1);
    moved.splice(to, 0, it);
    local = moved.map((p, i) => ({ ...p, featured_position: i + 1 }));
    try {
      await reorderFeatured(
        fetch,
        local.map((p) => p.id)
      );
    } catch (_e) {
      local = [...incoming];
    }
  }
  const moveLeft = (idx: number) => reorderTo(idx, idx - 1);
  const moveRight = (idx: number) => reorderTo(idx, idx + 1);
</script>

{#if local.length > 0 || isOwner}
  <section class="row" aria-label="Featured photos">
    {#each local as item, idx (item.id)}
      <div class="wrap">
        <FeaturedTile {item} {handle} />
        {#if editorMode}
          <div class="controls">
            <button
              type="button"
              class="ctl"
              disabled={idx === 0}
              onclick={() => moveLeft(idx)}
              aria-label="Move left">←</button
            >
            <button
              type="button"
              class="ctl"
              disabled={idx === local.length - 1}
              onclick={() => moveRight(idx)}
              aria-label="Move right">→</button
            >
            <button type="button" class="ctl" onclick={() => unpin(item.id)} aria-label="Unpin"
              >✕</button
            >
          </div>
        {/if}
      </div>
    {/each}
    {#each placeholders as i}
      <div class="slot">
        <span class="lab">SLOT {String(local.length + i + 1).padStart(2, '0')}</span>
        {#if i === 0 && editorMode}
          <button type="button" class="pin" onclick={() => (pinOpen = true)}>+ Pin a photo</button>
        {/if}
      </div>
    {/each}
  </section>
{/if}

{#if editorMode}
  <FeaturedPinModal
    bind:open={pinOpen}
    {handle}
    excludeIds={local.map((p) => p.id)}
    onPinned={(id) => onPinned(id)}
  />
{/if}

<style>
  .row {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 8px;
    padding: 16px 32px;
    border-top: 1px solid var(--border-subtle);
  }
  .wrap {
    position: relative;
  }
  .controls {
    position: absolute;
    top: 6px;
    right: 6px;
    z-index: 2;
    display: flex;
    gap: 4px;
  }
  .ctl {
    background: rgba(0, 0, 0, 0.55);
    color: #fff;
    border: 0;
    width: 24px;
    height: 24px;
    cursor: pointer;
    font-size: 12px;
  }
  .ctl:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .slot {
    aspect-ratio: 3 / 4;
    border: 1px dashed var(--border-subtle);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--fg-muted);
    font-family: var(--font-mono);
    font-size: 11px;
  }
  .pin {
    background: transparent;
    color: var(--accent);
    border: 1px solid var(--accent);
    padding: 6px 10px;
    font-family: var(--font-mono);
    font-size: 11px;
    cursor: pointer;
  }
  @media (max-width: 640px) {
    .row {
      grid-template-columns: repeat(2, 1fr);
    }
  }
</style>
