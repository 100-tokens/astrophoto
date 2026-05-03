<script lang="ts">
  import Img from '$lib/components/Img.svelte';
  import type { GalleryPhoto } from '$lib/api/GalleryPhoto';

  let {
    photos,
    handle
  }: {
    photos: GalleryPhoto[];
    handle: string;
  } = $props();
</script>

{#if photos.length > 0}
  <div class="strip">
    <p class="label">More from this photographer</p>
    <div class="grid">
      {#each photos as p (p.id)}
        <a
          class="thumb"
          href="/u/{handle}/p/{p.short_id}"
          aria-label={p.target ?? 'Untitled'}
        >
          <Img
            photoId={p.id}
            w={200}
            alt={p.target ?? 'Untitled'}
            aspectRatio="1/1"
            class="thumb-img"
          />
        </a>
      {/each}
    </div>
  </div>
{/if}

<style>
  .strip {
    padding: 16px 28px 24px;
    border-top: 1px solid var(--border-subtle);
  }

  .label {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-tertiary, var(--fg-secondary));
    margin: 0 0 12px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
  }

  .thumb {
    display: block;
    overflow: hidden;
    background: var(--bg-elevated);
    aspect-ratio: 1 / 1;
  }

  .thumb :global(.thumb-img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    transition: transform 0.2s ease;
  }

  .thumb:hover :global(.thumb-img) {
    transform: scale(1.05);
  }
</style>
