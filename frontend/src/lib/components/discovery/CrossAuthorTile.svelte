<script lang="ts">
  import type { DiscoveryPhoto } from '$lib/api/DiscoveryPhoto';
  import Img from '$lib/components/Img.svelte';
  import AuthorChip from './AuthorChip.svelte';
  import { cdn } from '$lib/cdn';
  import { openLightboxOnClick } from '$lib/util/openLightbox';
  import { timeAgoShort } from '$lib/util/relativeTime';

  let {
    photo,
    priority = false
  }: {
    photo: DiscoveryPhoto;
    /** First-row LCP candidate — eager-load + high fetchpriority. */
    priority?: boolean;
  } = $props();

  // Box aspect ratio drives the CSS-flex justified row. The browser reserves
  // height from `aspect-ratio` before the image loads, so there is no layout
  // shift, and server + client render identically (no JS measurement). Clamp
  // to the same range the old justified-layout used so panos/portraits stay
  // sane.
  const ROW_H = 240;
  let ar = $derived(Math.max(0.2, Math.min(5, (photo.width ?? 3) / (photo.height ?? 2))));
  // Nominal 1x render width, snapped to 80px buckets to limit CDN cache
  // fragmentation; srcset adds 2x/3x for retina.
  let nominalW = $derived(Math.min(1280, Math.max(160, Math.ceil((ar * ROW_H) / 80) * 80)));
  // Tiny LQIP shown behind the image while it loads (SSR/no-JS safe — it is a
  // CSS background, not a JS opacity toggle).
  let lqip = $derived(cdn(photo.id, { w: 32 }));
  let rel = $derived(photo.published_at ? timeAgoShort(photo.published_at) : '');
</script>

<!-- The tile is a multi-link card: the photo link and the caption's
     author link are SIBLINGS. The author chip used to live inside the
     tile anchor — nested <a> is invalid HTML, the parser closes the
     outer anchor early, and SvelteKit hydration crashed with
     HierarchyRequestError on every discovery route (explore, targets,
     categories, tags, search), falling back to a full client
     re-render. -->
<article class="tile" style="--ar:{ar}; flex-grow:{ar}; background-image:url('{lqip}');">
  <a
    use:openLightboxOnClick={{ handle: photo.author_handle, short_id: photo.short_id }}
    class="tile-link"
    href="/u/{photo.author_handle}/p/{photo.short_id}"
    aria-label={`${photo.target ?? photo.original_name ?? 'Untitled'} by @${photo.author_handle}`}
  >
    <Img photoId={photo.id} w={nominalW} sizes={`${nominalW}px`} {priority} alt="" class="img" />
  </a>
  <span class="cap">
    <span class="cap-left">
      <span class="title">{photo.target ?? photo.original_name ?? 'Untitled'}</span>
      <span class="meta">
        <AuthorChip handle={photo.author_handle} />
        {#if rel}<span class="ago">· {rel}{rel === 'NOW' ? '' : ' AGO'}</span>{/if}
      </span>
    </span>
    <span class="apps">♡ {photo.appreciations_count}</span>
  </span>
</article>

<style>
  .tile {
    position: relative;
    display: block;
    flex-basis: calc(var(--ar) * var(--row-h, 240px));
    aspect-ratio: var(--ar);
    min-width: 0;
    overflow: hidden;
    background-color: var(--bg-elevated);
    background-size: cover;
    background-position: center;
  }

  .tile-link {
    position: absolute;
    inset: 0;
    display: block;
  }

  .tile-link:focus-visible {
    /* Inset ring: the anchor fills the tile, whose overflow:hidden
       would clip an outward outline. */
    outline: 2px solid var(--accent);
    outline-offset: -2px;
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
    /* Clicks fall through to the photo link below; only the author
       chip (a sibling link, not a nested one) captures the pointer. */
    pointer-events: none;
    /* Always visible by default so touch / no-hover devices (which never get
       :hover) can read the caption. */
    opacity: 1;
    transition: opacity 0.15s ease-out;
  }

  .cap :global(.author-chip) {
    pointer-events: auto;
  }

  /* On hover-capable devices, keep the quiet gallery aesthetic: reveal the
     caption on hover or keyboard focus (per the original handoff).
     focus-within covers both the photo link and the author chip. */
  @media (hover: hover) {
    .cap {
      opacity: 0;
    }
    .tile:hover .cap,
    .tile:focus-within .cap {
      opacity: 1;
    }
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

  .meta {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .ago {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.04em;
    color: rgba(255, 255, 255, 0.7);
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
