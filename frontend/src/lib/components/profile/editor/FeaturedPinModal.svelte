<script lang="ts">
  import { pinFeatured } from '$lib/api/profileClient';
  import type { GalleryPhoto } from '$lib/api/GalleryPhoto';
  import PhotoPickerGrid from './PhotoPickerGrid.svelte';

  let {
    open = $bindable<boolean>(false),
    handle,
    excludeIds = [],
    onPinned = () => {}
  }: {
    open?: boolean;
    handle: string;
    excludeIds?: string[];
    onPinned?: (photoId: string) => void;
  } = $props();

  function close() {
    open = false;
  }

  async function pick(p: GalleryPhoto) {
    await pinFeatured(fetch, p.id);
    onPinned(p.id);
    close();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) close();
  }
</script>

<svelte:window onkeydown={onKey} />

{#if open}
  <div class="overlay" role="dialog" aria-modal="true" aria-label="Pin a photo">
    <button type="button" class="scrim" onclick={close} aria-label="Close"></button>
    <div class="dialog">
      <header>
        <h2>Pin a photo</h2>
        <button type="button" class="x" onclick={close} aria-label="Close">×</button>
      </header>
      <PhotoPickerGrid {handle} {excludeIds} onPick={(p) => void pick(p)} />
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: grid;
    place-items: center;
  }
  .scrim {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    border: 0;
    cursor: default;
  }
  .dialog {
    position: relative;
    width: 720px;
    max-width: 95vw;
    max-height: 80vh;
    overflow-y: auto;
    background: var(--bg-canvas);
    border: 1px solid var(--border-subtle);
    padding: 16px;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-subtle);
    padding-bottom: 12px;
    margin-bottom: 16px;
  }
  header h2 {
    margin: 0;
    font-family: var(--font-display, 'Source Serif 4', serif);
    font-weight: 400;
  }
  .x {
    background: transparent;
    color: var(--fg-muted);
    border: 0;
    font-size: 24px;
    cursor: pointer;
  }
</style>
