<script lang="ts">
  import type { FeaturedPhotoSummary } from '$lib/api/FeaturedPhotoSummary';
  import Img from '$lib/components/Img.svelte';

  let {
    cover,
    isOwner,
    onPickCover
  }: {
    cover: FeaturedPhotoSummary | null;
    isOwner: boolean;
    onPickCover: () => void;
  } = $props();

  let hasCover = $derived(cover !== null && cover !== undefined);
</script>

{#if hasCover}
  <header class="cover" aria-label="Cover photo">
    <Img photoId={cover!.id} w={2400} alt={cover!.target ?? 'Cover image'} class="cover-img" />
    <div class="cover-credit">
      <span class="dot">●</span>
      <span>COVER</span>
      {#if cover!.target}
        <span class="dim">·</span>
        <span>{cover!.target}</span>
      {/if}
    </div>
    {#if isOwner}
      <button type="button" class="cover-edit" onclick={onPickCover}>Change cover</button>
    {/if}
  </header>
{:else if isOwner}
  <header class="cover cover--empty">
    <button type="button" class="cover-prompt" onclick={onPickCover}>
      Pick a cover from your gallery →
    </button>
  </header>
{/if}

<style>
  .cover {
    position: relative;
    width: 100%;
    height: 480px;
    overflow: hidden;
    background: var(--bg-elevated);
  }
  .cover :global(.cover-img) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .cover::after {
    content: '';
    position: absolute;
    inset: auto 0 0 0;
    height: 30%;
    background: linear-gradient(to bottom, transparent, var(--bg-canvas));
    pointer-events: none;
  }
  .cover--empty {
    display: flex;
    align-items: center;
    justify-content: center;
    background: color-mix(in srgb, var(--accent) 4%, var(--bg-elevated));
    height: 240px;
  }
  .cover-credit {
    position: absolute;
    top: 16px;
    right: 24px;
    z-index: 1;
    display: flex;
    gap: 8px;
    align-items: center;
    color: #fff;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.06em;
  }
  .cover-credit .dot {
    color: var(--accent);
  }
  .cover-credit .dim {
    opacity: 0.6;
  }
  .cover-edit {
    position: absolute;
    top: 16px;
    left: 24px;
    z-index: 1;
    background: rgba(0, 0, 0, 0.5);
    color: #fff;
    border: 1px solid rgba(255, 255, 255, 0.2);
    padding: 6px 12px;
    font-family: var(--font-mono);
    font-size: 11px;
    cursor: pointer;
  }
  .cover-prompt {
    background: transparent;
    color: var(--accent);
    border: 1px solid var(--accent);
    padding: 12px 20px;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
  @media (max-width: 640px) {
    .cover {
      height: 28vh;
    }
  }
</style>
