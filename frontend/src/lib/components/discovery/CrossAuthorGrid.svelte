<script lang="ts">
  import type { DiscoveryPhoto } from '$lib/api/DiscoveryPhoto';
  import CrossAuthorTile from './CrossAuthorTile.svelte';
  import EmptyState from '$lib/components/EmptyState.svelte';

  let {
    initial = null,
    loadMore: loadMoreFn,
    emptyTitle = 'No frames here yet',
    emptyMessage = 'Nothing matches this view yet — be the first to publish a frame.'
  }: {
    initial?: { photos: DiscoveryPhoto[]; next_cursor: string | null } | null;
    loadMore?: () => Promise<{ photos: DiscoveryPhoto[]; next_cursor: string | null }>;
    emptyTitle?: string;
    emptyMessage?: string;
  } = $props();

  // Use extraPhotos + extraCursor so we never seed $state from a prop directly.
  let extraPhotos = $state<DiscoveryPhoto[]>([]);
  let extraCursor = $state<string | null>(null);
  let loading = $state(false);

  let photos = $derived([...(initial?.photos ?? []), ...extraPhotos]);
  let nextCursor = $derived(extraCursor !== null ? extraCursor : (initial?.next_cursor ?? null));

  // Justified-rows layout is pure CSS (flex-grow ∝ aspect-ratio, flex-basis ∝
  // aspect-ratio × row-height), so the full grid — real <a>/<img>/captions —
  // renders server-side and at first paint without JS (crawlable + LCP image
  // in the initial HTML), and server/client render identically (no re-layout,
  // no CLS). The trailing flex spacers keep the last (incomplete) row from
  // stretching its tiles to full width.
  const spacers = [0, 1, 2, 3, 4, 5];

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
</script>

<div class="grid">
  {#each photos as photo, i (photo.id)}
    <CrossAuthorTile {photo} priority={i < 3} />
  {/each}
  {#each spacers as s (s)}
    <i class="spacer" aria-hidden="true"></i>
  {/each}
</div>

{#if nextCursor && loadMoreFn}
  <div class="more">
    <button type="button" class="btn-more" disabled={loading} onclick={() => void loadMore()}>
      {loading ? 'Loading…' : 'Load more'}
    </button>
  </div>
{:else if photos.length === 0 && !loading}
  <EmptyState title={emptyTitle} message={emptyMessage} ctaLabel="Upload a frame" ctaHref="/upload" />
{/if}

<style>
  .grid {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 0 64px;
    --row-h: 240px;
  }

  .spacer {
    flex-grow: 1000;
    height: 0;
    margin: 0;
    padding: 0;
  }

  @media (max-width: 768px) {
    .grid {
      margin: 0 16px;
      --row-h: 160px;
    }
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
</style>
