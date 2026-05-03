<script lang="ts">
  import type { GalleryPhoto } from '$lib/api/GalleryPhoto';
  import { setCover } from '$lib/api/profileClient';
  import PhotoPickerGrid from './PhotoPickerGrid.svelte';

  let {
    open = $bindable<boolean>(false),
    handle,
    onPicked = () => {}
  }: {
    open?: boolean;
    handle: string;
    onPicked?: (photoId: string | null) => void;
  } = $props();

  function close() {
    open = false;
  }

  async function pick(p: GalleryPhoto) {
    await setCover(fetch, p.id);
    onPicked(p.id);
    close();
  }
  async function clear() {
    await setCover(fetch, null);
    onPicked(null);
    close();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) close();
  }
</script>

<svelte:window onkeydown={onKey} />

{#if open}
  <div class="overlay" role="dialog" aria-modal="true" aria-label="Pick a cover photo">
    <button type="button" class="scrim" aria-label="Close" onclick={close}></button>
    <div class="dialog">
      <header>
        <h2>Pick a cover</h2>
        <div class="actions">
          <button type="button" class="clear" onclick={() => void clear()}>Clear cover</button>
          <button type="button" class="x" onclick={close} aria-label="Close">×</button>
        </div>
      </header>
      <PhotoPickerGrid {handle} onPick={(p) => void pick(p)} />
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
  .actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .clear {
    background: transparent;
    border: 1px solid var(--border-subtle);
    color: var(--fg-secondary);
    padding: 6px 10px;
    font-family: var(--font-mono);
    font-size: 11px;
    cursor: pointer;
  }
  .x {
    background: transparent;
    color: var(--fg-muted);
    border: 0;
    font-size: 24px;
    cursor: pointer;
  }
</style>
