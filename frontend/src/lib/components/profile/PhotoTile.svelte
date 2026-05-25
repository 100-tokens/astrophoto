<script lang="ts">
  import type { GalleryPhoto } from '$lib/api/GalleryPhoto';
  import Img from '$lib/components/Img.svelte';
  import PhotoTitle from '$lib/components/photos/PhotoTitle.svelte';
  import { openLightboxOnClick } from '$lib/util/openLightbox';

  let {
    photo,
    handle,
    width,
    height,
    top,
    left
  }: {
    photo: GalleryPhoto;
    handle: string;
    width: number;
    height: number;
    top: number;
    left: number;
  } = $props();
</script>

<a
  use:openLightboxOnClick={{ handle, short_id: photo.short_id }}
  class="tile"
  style="width:{width}px; height:{height}px; transform: translate({left}px, {top}px);"
  href="/u/{handle}/p/{photo.short_id}"
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
    <PhotoTitle photo={{ target: photo.target }} size="md" />
    <span class="apps">{photo.appreciations_count} ❤</span>
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
    padding: 8px;
    background: linear-gradient(transparent, rgba(0, 0, 0, 0.55));
    color: #fff;
    font-family: var(--font-mono);
    font-size: 11px;
    display: flex;
    justify-content: space-between;
    opacity: 0;
    transition: opacity 0.15s ease-out;
  }
  .tile:hover .cap,
  .tile:focus-visible .cap {
    opacity: 1;
  }
</style>
