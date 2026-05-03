<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchPhotosFeed } from '$lib/api/profileClient';
  import type { GalleryPhoto } from '$lib/api/GalleryPhoto';
  import Img from '$lib/components/Img.svelte';

  let {
    handle,
    excludeIds = [],
    onPick
  }: {
    handle: string;
    excludeIds?: string[];
    onPick: (photo: GalleryPhoto) => void;
  } = $props();

  let photos = $state<GalleryPhoto[]>([]);
  let nextCursor = $state<string | null>(null);
  let loading = $state(false);

  let visible = $derived(photos.filter((p) => !excludeIds.includes(p.id)));

  onMount(() => {
    void load();
  });

  async function load() {
    if (loading) return;
    loading = true;
    try {
      const opts: { cursor?: string; limit?: number } = { limit: 24 };
      if (nextCursor) opts.cursor = nextCursor;
      const page = await fetchPhotosFeed(fetch, handle, opts);
      photos = [...photos, ...page.photos];
      nextCursor = page.next_cursor ?? null;
    } finally {
      loading = false;
    }
  }
</script>

<div class="picker">
  {#if visible.length === 0 && !loading}
    <p class="empty">No published photos to choose from yet.</p>
  {:else}
    <ul class="grid">
      {#each visible as p (p.id)}
        <li>
          <button type="button" class="cell" onclick={() => onPick(p)}>
            <Img
              photoId={p.id}
              w={300}
              aspectRatio="1/1"
              alt={p.target ?? 'Untitled'}
              class="img"
            />
            <span class="cap">{p.target ?? 'Untitled'}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
  {#if nextCursor}
    <button type="button" class="more" disabled={loading} onclick={() => void load()}>
      {loading ? 'Loading…' : 'Load more'}
    </button>
  {/if}
</div>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .grid {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
  }
  .cell {
    background: transparent;
    border: 1px solid var(--border-subtle);
    padding: 0;
    cursor: pointer;
    position: relative;
    aspect-ratio: 1 / 1;
    overflow: hidden;
  }
  .cell:hover {
    border-color: var(--accent);
  }
  .cell :global(.img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .cap {
    position: absolute;
    inset: auto 0 0 0;
    padding: 4px 6px;
    color: #fff;
    background: rgba(0, 0, 0, 0.55);
    font-family: var(--font-mono);
    font-size: 10px;
    text-align: left;
  }
  .empty {
    color: var(--fg-muted);
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .more {
    align-self: center;
    background: transparent;
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 6px 12px;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
</style>
