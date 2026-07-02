<script lang="ts">
  import { untrack } from 'svelte';
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

  // Seed the list state from the SSR-loaded initial page; subsequent pages
  // come in via the cursor + fetchPhotosFeed (see effect below). untrack
  // declares the prop read is intentional one-shot.
  let photos = $state<GalleryPhoto[]>(untrack(() => initial?.photos ?? []));
  let nextCursor = $state<string | null>(untrack(() => initial?.next_cursor ?? null));
  let loading = $state(false);

  // Justified-rows layout is pure CSS (flex-grow ∝ aspect-ratio,
  // flex-basis ∝ aspect-ratio × row-height) — the same pattern as the
  // explore grid (CrossAuthorGrid). The previous justified-layout
  // implementation measured the container in onMount, which never runs
  // during SSR, so the profile gallery server-rendered as an empty
  // <div style="height:0px"> — invisible to crawlers and no-JS readers
  // despite +page.server.ts SSR-loading the first page for exactly
  // that purpose. The trailing spacers keep the last (incomplete) row
  // from stretching its tiles to full width.
  const spacers = [0, 1, 2, 3, 4, 5];

  // Re-fetch when the user changes the sort. We react to the `sort` VALUE
  // only — never to `initial`'s object identity. An unrelated `invalidateAll()`
  // elsewhere on the page (avatar / cover / profile save re-run the page load)
  // hands us a brand-new `firstPage` object; if the effect depended on it,
  // each re-render would reset + re-fetch in a runaway loop (Svelte
  // `effect_update_depth_exceeded`, hammering `/photos`). `photos` is already
  // seeded once from `initial` at mount, so this component owns its list
  // afterwards and ignoring later `initial` changes is correct.
  let appliedSort = untrack(() => sort);
  $effect(() => {
    if (sort === appliedSort) return;
    appliedSort = sort;
    untrack(() => {
      photos = [];
      nextCursor = null;
      loading = false;
      void loadMore();
    });
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
</script>

<div class="grid">
  {#each photos as photo, i (photo.id)}
    <PhotoTile {photo} {handle} priority={i < 3} />
  {/each}
  {#each spacers as s (s)}
    <i class="spacer" aria-hidden="true"></i>
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
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin: 0 32px;
    --row-h: 220px;
  }

  .spacer {
    flex-grow: 1000;
    height: 0;
    margin: 0;
    padding: 0;
  }

  @media (max-width: 640px) {
    .grid {
      --row-h: 140px;
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
  .empty {
    padding: 48px 32px;
    color: var(--fg-muted);
    font-family: var(--font-mono);
    font-size: 12px;
  }
</style>
