<script lang="ts">
  import type { GalleryPhoto } from '$lib/api/GalleryPhoto';
  import Img from '$lib/components/Img.svelte';
  import PhotoTitle from '$lib/components/photos/PhotoTitle.svelte';
  import { openLightboxOnClick } from '$lib/util/openLightbox';

  let {
    photo,
    handle,
    priority = false
  }: {
    photo: GalleryPhoto;
    handle: string;
    /** First-row LCP candidate — eager-load + high fetchpriority. */
    priority?: boolean;
  } = $props();

  // Aspect ratio drives the CSS-flex justified row (see PhotoGrid): the
  // browser reserves height from `aspect-ratio` before the image loads,
  // and server + client render identically — no JS measurement, so the
  // tile exists in the SSR HTML. Clamp to the same range the old
  // justified-layout used so panos/portraits stay sane.
  const ROW_H = 220;
  let ar = $derived(Math.max(0.2, Math.min(5, (photo.width ?? 3) / (photo.height ?? 2))));
  // Nominal 1x render width, snapped to 80px buckets to limit CDN cache
  // fragmentation; srcset adds 2x/3x for retina.
  let nominalW = $derived(Math.min(1280, Math.max(160, Math.ceil((ar * ROW_H) / 80) * 80)));
</script>

<a
  use:openLightboxOnClick={{ handle, short_id: photo.short_id }}
  class="tile"
  style="--ar:{ar}; flex-grow:{ar};"
  href="/u/{handle}/p/{photo.short_id}"
  aria-label={photo.target ?? 'Untitled'}
>
  <Img
    photoId={photo.id}
    w={nominalW}
    sizes={`${nominalW}px`}
    {priority}
    alt={photo.target ?? 'Untitled'}
    class="img"
  />
  <span class="cap">
    <PhotoTitle photo={{ target: photo.target, original_name: photo.original_name }} size="md" />
    <span class="apps">{photo.appreciations_count} ❤</span>
  </span>
</a>

<style>
  .tile {
    position: relative;
    display: block;
    flex-basis: calc(var(--ar) * var(--row-h, 220px));
    aspect-ratio: var(--ar);
    min-width: 0;
    overflow: hidden;
    background: var(--bg-elevated);
  }
  .tile:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
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
