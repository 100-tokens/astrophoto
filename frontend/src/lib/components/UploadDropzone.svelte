<script lang="ts">
  interface Props {
    onFiles: (files: File[]) => void;
    tierMax?: number;
    overQuota?: boolean;
  }

  let { onFiles, overQuota = false }: Props = $props();

  let dragOver = $state(false);
  let inputEl: HTMLInputElement;

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    const files = Array.from(e.dataTransfer?.files ?? []);
    if (files.length) onFiles(files);
  }

  function openPicker() {
    // Clear value so re-selecting the same file re-fires onchange.
    inputEl.value = '';
    inputEl.click();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      openPicker();
    }
  }
</script>

<div
  class="dz"
  class:dz-drag={dragOver}
  class:dz-disabled={overQuota}
  ondragover={(e) => {
    e.preventDefault();
    dragOver = true;
  }}
  ondragleave={() => {
    dragOver = false;
  }}
  ondrop={handleDrop}
  onclick={openPicker}
  onkeydown={handleKeydown}
  role="button"
  tabindex="0"
  aria-label="Drop photos to upload"
  aria-disabled={overQuota}
>
  <p class="dz-headline t-display">↑ Drop photos here, or click</p>
  <p class="t-meta">JPEG · PNG · TIFF · up to 50 MB (free) · Subscribers up to 200 MB</p>
  <input
    bind:this={inputEl}
    type="file"
    multiple
    accept="image/jpeg,image/png,image/tiff"
    style="display:none"
    onchange={(e) => {
      const fs = Array.from((e.target as HTMLInputElement).files ?? []);
      if (fs.length) onFiles(fs);
    }}
  />
</div>

<style>
  .dz {
    padding: 64px 32px;
    border: 1px dashed var(--border-default);
    text-align: center;
    cursor: pointer;
    transition:
      border-color 0.15s,
      background 0.15s;
    border-radius: var(--r-md, 4px);
    outline-offset: 2px;
  }

  .dz:focus-visible {
    outline: 2px solid var(--accent);
  }

  .dz-drag {
    border-color: var(--accent);
    background: color-mix(in oklab, var(--accent) 7%, transparent);
  }

  .dz-disabled {
    border-color: var(--warning);
    background: color-mix(in oklab, var(--warning) 8%, transparent);
    cursor: not-allowed;
  }

  .dz-headline {
    font-size: 22px;
    font-style: italic;
    margin: 0 0 8px;
  }
</style>
