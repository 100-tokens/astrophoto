<script lang="ts">
  import type { FeaturedPhotoSummary } from '$lib/api/FeaturedPhotoSummary';
  import Img from '$lib/components/Img.svelte';

  let {
    item,
    handle
  }: {
    item: FeaturedPhotoSummary;
    handle: string;
  } = $props();
</script>

<a
  class="tile"
  href="/u/{handle}/p/{item.short_id}"
  aria-label={item.target ?? 'Featured photo'}
>
  <span class="rank">#{String(item.featured_position).padStart(2, '0')}</span>
  <Img
    photoId={item.id}
    w={600}
    aspectRatio="3/4"
    alt={item.target ?? 'Featured photo'}
    class="img"
  />
  <span class="cap">
    <span class="target">{item.target ?? 'Untitled'}</span>
    <span class="apps">{item.appreciations_count} ❤</span>
  </span>
</a>

<style>
  .tile {
    position: relative;
    display: block;
    aspect-ratio: 3 / 4;
    overflow: hidden;
    background: var(--bg-elevated);
  }
  .tile :global(.img) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .rank {
    position: absolute;
    top: 8px;
    left: 8px;
    z-index: 1;
    color: #fff;
    font-family: var(--font-mono);
    font-size: 11px;
    background: rgba(0, 0, 0, 0.45);
    padding: 2px 6px;
  }
  .cap {
    position: absolute;
    inset: auto 0 0 0;
    padding: 8px;
    background: linear-gradient(transparent, rgba(0, 0, 0, 0.6));
    color: #fff;
    font-family: var(--font-mono);
    font-size: 11px;
    display: flex;
    justify-content: space-between;
  }
</style>
