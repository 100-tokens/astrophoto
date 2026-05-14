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
  // Detect Mac vs others so the paste-hint chip renders the right modifier.
  const pasteHint = $derived.by(() => {
    if (typeof navigator === 'undefined') return '⌘V';
    return /mac/i.test(navigator.platform || navigator.userAgent) ? '⌘V' : 'Ctrl+V';
  });

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    if (overQuota) return;
    const files = Array.from(e.dataTransfer?.files ?? []);
    if (files.length) onFiles(files);
  }

  // Paste-from-clipboard. Listens at window-scope while the dropzone is
  // mounted, but skips when the user is typing in an editable element so
  // pasting text into a tag input doesn't accidentally enqueue an image.
  $effect(() => {
    if (overQuota) return;
    function isEditable(t: EventTarget | null): boolean {
      if (!(t instanceof HTMLElement)) return false;
      const tag = t.tagName;
      return (
        tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || t.isContentEditable === true
      );
    }
    function onPaste(e: ClipboardEvent) {
      if (isEditable(e.target)) return;
      const items = e.clipboardData?.items;
      if (!items) return;
      const files: File[] = [];
      for (const item of items) {
        if (item.kind === 'file' && item.type.startsWith('image/')) {
          const f = item.getAsFile();
          if (f) files.push(f);
        }
      }
      if (files.length) {
        e.preventDefault();
        onFiles(files);
      }
    }
    window.addEventListener('paste', onPaste);
    return () => window.removeEventListener('paste', onPaste);
  });
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
  <span class="dz-paste-hint t-meta" aria-hidden="true">{pasteHint} to paste from clipboard</span>
  <svg
    class="dz-icon"
    width="40"
    height="40"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.2"
    aria-hidden="true"
  >
    <path d="M12 16V4M6 10l6-6 6 6M4 20h16" />
  </svg>
  <span class="dz-headline t-display">Drop photos here, or click to browse</span>
  <span class="t-meta dz-sub">
    JPEG · PNG · TIFF (16-bit) &nbsp;·&nbsp; up to {tier === 'subscriber' ? '200 MB' : '50 MB'} per file
    &nbsp;·&nbsp; up to 12 at once
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
    padding: 56px 32px;
    border: 1px dashed var(--accent);
    background: color-mix(in oklab, var(--accent) 5%, transparent);
    color: var(--accent);
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
  .dz-icon {
    color: var(--accent);
    margin-bottom: 8px;
  }
  .dz-sub {
    color: var(--fg-secondary);
  }
  .dz-paste-hint {
    position: absolute;
    top: 12px;
    right: 12px;
    padding: 4px 8px;
    border: 1px solid color-mix(in oklab, var(--accent) 40%, transparent);
    border-radius: 3px;
    color: var(--accent);
    font-size: 10px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    pointer-events: none;
    background: color-mix(in oklab, var(--bg-canvas) 92%, transparent);
  }
  @media (max-width: 640px) {
    .dz {
      padding: 32px 16px;
    }
    .dz-paste-hint {
      display: none;
    }
    .dz-headline {
      font-size: 18px;
    }
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
    color: var(--accent);
  }
</style>
