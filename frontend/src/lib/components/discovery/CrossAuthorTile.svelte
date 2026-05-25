<script lang="ts">
  import type { DiscoveryPhoto } from '$lib/api/DiscoveryPhoto';
  import Img from '$lib/components/Img.svelte';
  import AuthorChip from './AuthorChip.svelte';
  import { openLightboxOnClick } from '$lib/util/openLightbox';

  let {
    photo,
    width,
    height,
    top,
    left
  }: {
    photo: DiscoveryPhoto;
    width: number;
    height: number;
    top: number;
    left: number;
  } = $props();
</script>

<a
  use:openLightboxOnClick={{ handle: photo.author_handle, short_id: photo.short_id }}
  class="tile"
  style="width:{width}px; height:{height}px; transform: translate({left}px, {top}px);"
  href="/u/{photo.author_handle}/p/{photo.short_id}"
  aria-label={photo.target ?? 'Untitled'}
>
  <Img
    photoId={photo.id}
    w={Math.round(width)}
    sizes={`${Math.round(width)}px`}
    alt={photo.target ?? 'Untitled'}
    class="img"
  />
  <span class="cap">
    <span class="cap-left">
      <span class="title">{photo.target ?? 'Untitled'}</span>
      <AuthorChip handle={photo.author_handle} />
    </span>
    <span class="apps">♡ {photo.appreciations_count}</span>
  </span>
</a>

<style>
  .tile {
    position: absolute;
    top: 0;
    left: 0;
    overflow: hidden;
    background: var(--bg-elevated);
    transform-origin: top left;
  }

  .tile :global(.img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .cap {
    position: absolute;
    inset: auto 0 0 0;
    padding: 10px 10px 8px;
    background: linear-gradient(to top, rgba(0, 0, 0, 0.78), transparent 90%);
    color: #fff;
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 8px;
    opacity: 0;
    transition: opacity 0.15s ease-out;
  }

  .tile:hover .cap,
  .tile:focus-visible .cap {
    opacity: 1;
  }

  .cap-left {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .title {
    font-family: var(--font-display);
    font-size: 14px;
    font-style: italic;
    color: #fff;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .apps {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--accent);
    background: rgba(12, 10, 8, 0.7);
    border: 1px solid var(--accent-dim);
    padding: 2px 6px;
    white-space: nowrap;
    flex-shrink: 0;
  }
</style>
