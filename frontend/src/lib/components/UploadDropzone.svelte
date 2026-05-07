<script lang="ts">
  interface Props {
    onFiles: (files: File[]) => void;
    tierMax?: number;
    tier?: 'free' | 'subscriber';
    overQuota?: boolean;
  }

  let { onFiles, tier = 'free', overQuota = false }: Props = $props();

  let dragOver = $state(false);
  // Stable id so the wrapping <label> can target the hidden input even when
  // multiple <UploadDropzone> instances coexist on a page.
  const inputId = `upload-dz-input-${crypto.randomUUID()}`;

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    if (overQuota) return;
    const files = Array.from(e.dataTransfer?.files ?? []);
    if (files.length) onFiles(files);
  }
</script>

<!--
  Wrapping the surface in a <label for=…> is the most reliable way to
  forward a click anywhere inside the dropzone to the hidden file input.
  The previous implementation called inputEl.click() from an onclick on a
  <div role="button"> — Chrome/Safari sometimes refuse to open the OS file
  dialog from a synthetic click on a `display:none` input under strict
  user-activation rules. The label-based pattern sidesteps that entirely.
-->
<label
  class="dz"
  class:dz-drag={dragOver}
  class:dz-disabled={overQuota}
  for={inputId}
  ondragover={(e) => {
    e.preventDefault();
    dragOver = true;
  }}
  ondragleave={() => {
    dragOver = false;
  }}
  ondrop={handleDrop}
  aria-disabled={overQuota}
>
  <span class="dz-headline t-display">↑ Drop photos here, or click</span>
  <span class="t-meta">
    JPEG · PNG · TIFF · up to {tier === 'subscriber' ? '200 MB' : '50 MB (free)'}
    {#if tier !== 'subscriber'}
      · Subscribers up to 200 MB{/if}
  </span>
  <input
    id={inputId}
    type="file"
    multiple
    accept="image/jpeg,image/png,image/tiff"
    class="dz-input"
    disabled={overQuota}
    onchange={(e) => {
      const target = e.target as HTMLInputElement;
      const fs = Array.from(target.files ?? []);
      // Reset so re-selecting the same file re-fires onchange.
      target.value = '';
      if (fs.length) onFiles(fs);
    }}
  />
</label>

<style>
  .dz {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
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

  .dz:focus-within {
    outline: 2px solid var(--accent);
  }

  /* Visually-hidden but focusable + activatable; the wrapping <label> fully
     forwards clicks. `display: none` would break the click forwarding. */
  .dz-input {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
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
  }
</style>
