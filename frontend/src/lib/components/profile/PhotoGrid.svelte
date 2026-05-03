<script lang="ts">
  import { onMount } from 'svelte';
  import justifiedLayout from 'justified-layout';
  import type { GalleryPhoto } from '$lib/api/GalleryPhoto';
  import { fetchPhotosFeed } from '$lib/api/profileClient';
  import PhotoTile from './PhotoTile.svelte';

  let {
    handle,
    initial = null,
    sort = 'newest'
  }: {
    handle: string;
    initial?: { photos: GalleryPhoto[]; next_cursor: string | null } | null;
    sort?: 'newest' | 'popular';
  } = $props();

  let photos = $state<GalleryPhoto[]>(initial?.photos ?? []);
  let nextCursor = $state<string | null>(initial?.next_cursor ?? null);
  let loading = $state(false);
  let containerWidth = $state(0);
  let containerEl: HTMLDivElement | null = null;

  // Re-fetch when sort changes (skip the very first run if `initial` is set).
  let firstRun = true;
  $effect(() => {
    const _ = sort; // dependency
    if (firstRun && (initial?.photos.length ?? 0) > 0) {
      firstRun = false;
      return;
    }
    firstRun = false;
    photos = [];
    nextCursor = null;
    loading = false;
    void loadMore();
  });

  async function loadMore() {
    if (loading) return;
    loading = true;
    try {
      const opts: { cursor?: string; sort?: 'newest' | 'popular'; limit?: number } = {
        sort,
        limit: 24
      };
      if (nextCursor) opts.cursor = nextCursor;
      const page = await fetchPhotosFeed(fetch, handle, opts);
      photos = [...photos, ...page.photos];
      nextCursor = page.next_cursor ?? null;
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    if (containerEl) {
      containerWidth = containerEl.getBoundingClientRect().width;
      const ro = new ResizeObserver((entries) => {
        for (const e of entries) containerWidth = e.contentRect.width;
      });
      ro.observe(containerEl);
      return () => ro.disconnect();
    }
  });

  let layout = $derived.by(() => {
    if (containerWidth <= 0 || photos.length === 0) {
      return { containerHeight: 0, boxes: [] as Array<{ width: number; height: number; top: number; left: number }> };
    }
    const isMobile = containerWidth < 640;
    const aspectRatios = photos.map((p) => {
      const w = p.width ?? 3;
      const h = p.height ?? 2;
      return Math.max(0.2, Math.min(5, w / h));
    });
    const result = justifiedLayout(aspectRatios, {
      containerWidth,
      containerPadding: 0,
      boxSpacing: 8,
      targetRowHeight: isMobile ? 140 : 220
    });
    return result;
  });
</script>

<div class="grid" bind:this={containerEl} style="height:{layout.containerHeight}px">
  {#each photos as photo, i (photo.id)}
    {#if layout.boxes[i]}
      <PhotoTile
        {photo}
        {handle}
        width={layout.boxes[i].width}
        height={layout.boxes[i].height}
        top={layout.boxes[i].top}
        left={layout.boxes[i].left}
      />
    {/if}
  {/each}
</div>

{#if nextCursor}
  <div class="more">
    <button type="button" class="btn-more" disabled={loading} onclick={() => void loadMore()}>
      {loading ? 'Loading…' : 'Load more'}
    </button>
  </div>
{:else if photos.length === 0 && !loading}
  <p class="empty">No photos yet.</p>
{/if}

<style>
  .grid {
    position: relative;
    margin: 0 32px;
  }
  .more {
    display: flex;
    justify-content: center;
    padding: 24px;
  }
  .btn-more {
    background: transparent;
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 8px 16px;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
  .empty {
    padding: 48px 32px;
    color: var(--fg-muted);
    font-family: var(--font-mono);
    font-size: 12px;
  }
</style>
