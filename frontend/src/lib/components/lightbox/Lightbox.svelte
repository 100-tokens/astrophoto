<script lang="ts">
  import { onMount } from 'svelte';
  import ZoomableImage from '$lib/components/photos/ZoomableImage.svelte';
  import LightboxExifPanel from './LightboxExifPanel.svelte';
  import MoreFromPhotographerStrip from './MoreFromPhotographerStrip.svelte';
  import type { PhotoDetail } from '$lib/api/types';
  import type { GalleryPhoto } from '$lib/api/GalleryPhoto';

  let {
    photo,
    handle,
    morePhotos = [],
    onClose,
    onPrev,
    onNext
  }: {
    photo: PhotoDetail;
    handle: string;
    morePhotos?: GalleryPhoto[];
    onClose: () => void;
    onPrev?: (() => void) | undefined;
    onNext?: (() => void) | undefined;
  } = $props();

  let showExif = $state(true);

  let title = $derived(photo.target ?? photo.original_name);

  function onKeydown(e: KeyboardEvent) {
    switch (e.key) {
      case 'Escape':
        onClose();
        break;
      case 'ArrowLeft':
        if (onPrev) onPrev();
        break;
      case 'ArrowRight':
        if (onNext) onNext();
        break;
      case 'i':
      case 'I':
        showExif = !showExif;
        break;
      case 'a':
      case 'A': {
        const btn = document.querySelector<HTMLButtonElement>('[data-appreciate-trigger]');
        btn?.click();
        break;
      }
    }
  }

  onMount(() => {
    // Prevent body scroll while lightbox is open
    document.body.style.overflow = 'hidden';
    return () => {
      document.body.style.overflow = '';
    };
  });
</script>

<svelte:window onkeydown={onKeydown} />

<!-- Backdrop -->
<div class="backdrop" role="presentation" onclick={onClose}></div>

<!-- Overlay -->
<div class="lightbox" role="dialog" aria-modal="true" aria-label={title}>
  <!-- Top bar -->
  <div class="topbar">
    <button class="close-btn" type="button" aria-label="Close lightbox" onclick={onClose}>
      ×
    </button>
  </div>

  <!-- Main content grid -->
  <div class="content" class:no-exif={!showExif}>
    <!-- Left: image area -->
    <div class="image-area">
      {#if onPrev}
        <button class="arrow arrow-prev" type="button" aria-label="Previous photo" onclick={onPrev}>
          ‹
        </button>
      {/if}

      <div class="image-wrap">
        <ZoomableImage photoId={photo.id} alt={title} w={2400} maxHeight="calc(100vh - 96px)" />
      </div>

      {#if onNext}
        <button class="arrow arrow-next" type="button" aria-label="Next photo" onclick={onNext}>
          ›
        </button>
      {/if}
    </div>

    <!-- Right: EXIF panel + more-from strip -->
    {#if showExif}
      <div class="side-panel">
        <LightboxExifPanel {photo} />
        <MoreFromPhotographerStrip photos={morePhotos} {handle} />
      </div>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.88);
    z-index: 200;
  }

  .lightbox {
    position: fixed;
    inset: 0;
    z-index: 201;
    display: flex;
    flex-direction: column;
    pointer-events: none;
  }

  .topbar {
    display: flex;
    justify-content: flex-end;
    padding: 12px 16px;
    pointer-events: auto;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #fff;
    font-size: 28px;
    line-height: 1;
    cursor: pointer;
    padding: 4px 8px;
    opacity: 0.8;
    transition: opacity 0.15s;
  }
  .close-btn:hover {
    opacity: 1;
  }

  .content {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr 380px;
    overflow: hidden;
    pointer-events: auto;
  }

  .content.no-exif {
    grid-template-columns: 1fr;
  }

  .image-area {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    min-height: 0;
  }

  .image-wrap {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0 48px;
    box-sizing: border-box;
  }

  .image-wrap :global(.viewer) {
    max-width: 100%;
  }

  .arrow {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    background: rgba(0, 0, 0, 0.4);
    border: none;
    color: #fff;
    font-size: 36px;
    line-height: 1;
    cursor: pointer;
    width: 44px;
    height: 64px;
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1;
    transition: background 0.15s;
    border-radius: 2px;
  }

  .arrow:hover {
    background: rgba(0, 0, 0, 0.65);
  }

  .arrow-prev {
    left: 8px;
  }

  .arrow-next {
    right: 8px;
  }

  .side-panel {
    background: var(--bg-surface, #111);
    border-left: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  /* Mobile: stack vertically */
  @media (max-width: 768px) {
    .content {
      grid-template-columns: 1fr;
      grid-template-rows: auto 1fr;
    }

    .content.no-exif {
      grid-template-rows: 1fr;
    }

    .side-panel {
      border-left: none;
      border-top: 1px solid var(--border-subtle);
      overflow-y: auto;
      max-height: 40vh;
    }

    .image-wrap {
      padding: 0 40px;
    }
  }
</style>
