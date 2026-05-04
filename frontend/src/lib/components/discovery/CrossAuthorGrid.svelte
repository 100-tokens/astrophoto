<script lang="ts">
  import { onMount } from 'svelte';
  import justifiedLayout from 'justified-layout';
  import type { DiscoveryPhoto } from '$lib/api/DiscoveryPhoto';
  import CrossAuthorTile from './CrossAuthorTile.svelte';

  let {
    initial = null,
    loadMore: loadMoreFn
  }: {
    initial?: { photos: DiscoveryPhoto[]; next_cursor: string | null } | null;
    loadMore?: () => Promise<{ photos: DiscoveryPhoto[]; next_cursor: string | null }>;
  } = $props();

  // Use extraPhotos + extraCursor so we never seed $state from a prop directly.
  // This avoids state_referenced_locally warning without an eslint exemption.
  let extraPhotos = $state<DiscoveryPhoto[]>([]);
  let extraCursor = $state<string | null>(null);
  let loading = $state(false);
  let containerWidth = $state(0);
  let containerEl: HTMLDivElement | null = null;

  let photos = $derived([...(initial?.photos ?? []), ...extraPhotos]);
  let nextCursor = $derived(extraCursor !== null ? extraCursor : (initial?.next_cursor ?? null));

  async function loadMore() {
    if (loading || !loadMoreFn) return;
    loading = true;
    try {
      const page = await loadMoreFn();
      extraPhotos = [...extraPhotos, ...page.photos];
      extraCursor = page.next_cursor;
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
      return {
        containerHeight: 0,
        boxes: [] as Array<{ width: number; height: number; top: number; left: number }>
      };
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
      targetRowHeight: isMobile ? 140 : 240
    });
    return result;
  });
</script>

<div class="grid" bind:this={containerEl} style="height:{layout.containerHeight}px">
  {#each photos as photo, i (photo.id)}
    {#if layout.boxes[i]}
      <CrossAuthorTile
        {photo}
        width={layout.boxes[i].width}
        height={layout.boxes[i].height}
        top={layout.boxes[i].top}
        left={layout.boxes[i].left}
      />
    {/if}
  {/each}
</div>

{#if nextCursor && loadMoreFn}
  <div class="more">
    <button type="button" class="btn-more" disabled={loading} onclick={() => void loadMore()}>
      {loading ? 'Loading…' : 'Load more'}
    </button>
  </div>
{:else if photos.length === 0 && !loading}
  <p class="empty">No photos yet — be the first to upload.</p>
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

  .btn-more:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .empty {
    padding: 48px 32px;
    color: var(--fg-muted);
    font-family: var(--font-mono);
    font-size: 12px;
  }
</style>
