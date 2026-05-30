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

  // a11y: the Lightbox is always a modal — every consumer renders it gated on
  // the shallow-route `page.state.lightbox` flag (LightboxHost over a feed, or
  // the permalink page when reached via shallow nav). The standalone full-page
  // permalink renders PhotoDetailFull instead, so there is no non-modal branch
  // here to exclude. Capture/restore focus and trap Tab like Modal.svelte.
  let dialogEl: HTMLDivElement = $state() as HTMLDivElement;
  let closeBtnEl: HTMLButtonElement = $state() as HTMLButtonElement;
  let invokerBefore: HTMLElement | null = null;

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
      case 'Tab': {
        // Trap focus within the dialog (cycle first <-> last focusable).
        if (!dialogEl) return;
        const focusables = dialogEl.querySelectorAll<HTMLElement>(
          'a, button, input, textarea, select, [tabindex]:not([tabindex="-1"])'
        );
        if (!focusables.length) return;
        const first = focusables[0] as HTMLElement | undefined;
        const last = focusables[focusables.length - 1] as HTMLElement | undefined;
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last?.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first?.focus();
        }
        break;
      }
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

    // Capture the element that opened the lightbox (the originating tile link)
    // so focus can be restored to it on close.
    invokerBefore = document.activeElement as HTMLElement | null;

    // Mark the page content behind the modal inert. The dialog is nested
    // somewhere inside the page tree (not a direct <body> child), so we walk up
    // from the dialog to <body> and inert every sibling along the way — i.e.
    // everything except the dialog's own ancestor chain. This needs no
    // reference to <main>/header/footer, so it works without a refactor.
    const inerted: HTMLElement[] = [];
    // Guard against a same-tick mount→unmount: if cleanup runs before this
    // microtask flushes, skip the inert walk entirely so no `inert` leaks onto
    // the page (which would freeze it for everyone).
    let cancelled = false;
    queueMicrotask(() => {
      if (cancelled) return;
      // The backdrop is the dialog's immediate previous sibling and must stay
      // interactive (click-to-close), so it is never inerted.
      const backdrop = dialogEl?.previousElementSibling ?? null;
      let node: HTMLElement | null = dialogEl;
      while (node && node !== document.body) {
        const parentEl: HTMLElement | null = node.parentElement;
        if (!parentEl) break;
        for (const sibling of Array.from(parentEl.children)) {
          if (sibling === node || sibling === backdrop) continue;
          if (!(sibling instanceof HTMLElement)) continue;
          if (sibling.hasAttribute('inert')) continue;
          // Keep live regions announcing — prev/next call goto() (a real route
          // change), and SvelteKit's #svelte-announcer must stay reachable so
          // screen-reader users hear the new page title.
          if (sibling.id === 'svelte-announcer' || sibling.hasAttribute('aria-live')) continue;
          sibling.setAttribute('inert', '');
          inerted.push(sibling);
        }
        node = parentEl;
      }
      // Focus the close button once the dialog is in the DOM.
      closeBtnEl?.focus();
    });

    return () => {
      cancelled = true;
      document.body.style.overflow = '';
      for (const el of inerted) el.removeAttribute('inert');
      // Restore focus to the originating tile link.
      invokerBefore?.focus();
    };
  });
</script>

<svelte:window onkeydown={onKeydown} />

<!-- Backdrop -->
<div class="backdrop" role="presentation" onclick={onClose}></div>

<!-- Overlay -->
<div class="lightbox" role="dialog" aria-modal="true" aria-label={title} bind:this={dialogEl}>
  <!-- Top bar -->
  <div class="topbar">
    <button
      class="close-btn"
      type="button"
      aria-label="Close lightbox"
      onclick={onClose}
      bind:this={closeBtnEl}
    >
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
    /* WCAG 2.5.8: target >= 24x24px. Use a 44px box for a comfortable hit
       area and center the glyph. */
    min-width: 44px;
    min-height: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4px 8px;
    opacity: 0.8;
    transition: opacity 0.15s;
  }
  .close-btn:hover {
    opacity: 1;
  }
  .close-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
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
