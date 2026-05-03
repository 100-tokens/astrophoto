<script lang="ts">
  import { browser } from '$app/environment';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import type { ActionData } from './$types';

  let { form }: { form: ActionData } = $props();

  // Steps for the visual stepper (MVP: all three shown as chrome)
  const STEPS: Array<{ n: string; label: string; state: 'done' | 'active' | '' }> = [
    { n: '01', label: 'UPLOAD', state: 'active' },
    { n: '02', label: 'VERIFY DATA', state: '' },
    { n: '03', label: 'CAPTION & PUBLISH', state: '' }
  ];

  // File preview state
  let previewUrl = $state<string | null>(null);
  let fileName = $state<string | null>(null);
  let fileSize = $state<string | null>(null);

  function formatBytes(n: number): string {
    if (n >= 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(1)} MB`;
    if (n >= 1024) return `${(n / 1024).toFixed(1)} KB`;
    return `${n} B`;
  }

  function handleFileChange(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    if (browser) {
      if (previewUrl) URL.revokeObjectURL(previewUrl);
      previewUrl = URL.createObjectURL(file);
    }
    fileName = file.name;
    fileSize = formatBytes(file.size);
  }
</script>

<AppHeader active="Gallery" />

<div class="upload-page">
  <!-- Page header + stepper -->
  <section class="page-header">
    <div class="t-eyebrow">NEW FRAME</div>
    <h1 class="page-title">Add a <em>frame</em> to your archive</h1>

    <!-- 3-step stepper (visual chrome only for MVP) -->
    <div class="stepper">
      {#each STEPS as step}
        <div
          class="step"
          style="border-top-color: {step.state
            ? 'var(--accent)'
            : 'var(--border-default)'}; color: {step.state
            ? 'var(--fg-primary)'
            : 'var(--fg-muted)'};"
        >
          <span style="color: {step.state ? 'var(--accent)' : 'var(--fg-faint)'};">{step.n}</span>
          <span>{step.label}</span>
          {#if step.state === 'done'}
            <span class="step-check">✓</span>
          {/if}
        </div>
      {/each}
    </div>
  </section>

  <!-- Main form -->
  <section class="form-section">
    <form method="POST" enctype="multipart/form-data" class="upload-form">
      <!-- Left: drop zone / file input + preview -->
      <div class="col-left">
        <div class="t-label" style="margin-bottom: 12px;">YOUR UPLOAD</div>

        {#if previewUrl}
          <!-- Image preview -->
          <div class="preview-wrap">
            <img
              src={previewUrl}
              alt="Preview of {fileName ?? 'uploaded file'}"
              class="preview-img"
            />
          </div>
          {#if fileName && fileSize}
            <div class="file-meta">
              <span>{fileName}</span>
              <span>{fileSize}</span>
            </div>
          {/if}
          <!-- Allow re-selecting -->
          <label
            class="btn btn-secondary btn-sm"
            style="margin-top: 12px; cursor: pointer; display: inline-block;"
          >
            Replace file
            <input
              type="file"
              name="file"
              accept="image/jpeg,image/png,image/tiff"
              onchange={handleFileChange}
              style="display: none;"
            />
          </label>
        {:else}
          <!-- Drop zone -->
          <label class="drop-zone">
            <input
              type="file"
              name="file"
              accept="image/jpeg,image/png,image/tiff"
              onchange={handleFileChange}
              required
            />
            <div class="drop-zone-inner">
              <div class="drop-icon" aria-hidden="true">+</div>
              <div class="drop-title">Choose a file</div>
              <div class="drop-sub">JPEG · PNG · TIFF · up to 50 MB</div>
            </div>
          </label>
        {/if}
      </div>

      <!-- Error message -->
      {#if form?.message}
        <div class="error-banner" role="alert">
          {form.message}
        </div>
      {/if}

      <div class="actions">
        <a href="/" class="btn btn-ghost btn-lg">Cancel</a>
        <button type="submit" class="btn btn-primary btn-lg">Continue →</button>
      </div>
    </form>
  </section>
</div>

<style>
  .upload-page {
    min-height: calc(100dvh - 64px);
    background: var(--bg-base);
  }

  /* ── Page header ──────────────────────────────────────────── */
  .page-header {
    padding: 40px 64px 24px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .page-title {
    font-family: var(--font-display);
    font-size: 48px;
    font-weight: 400;
    margin: 8px 0 0;
    line-height: 1;
  }

  /* ── Stepper ──────────────────────────────────────────────── */
  .stepper {
    display: flex;
    margin-top: 32px;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .step {
    flex: 1;
    padding: 16px 0;
    border-top: 2px solid var(--border-default);
    display: flex;
    gap: 12px;
    align-items: center;
  }

  .step-check {
    color: var(--accent);
    margin-left: auto;
    margin-right: 32px;
  }

  /* ── Form section ─────────────────────────────────────────── */
  .form-section {
    padding: 48px 64px;
  }

  .upload-form {
    display: grid;
    grid-template-columns: 1fr;
    max-width: 640px;
  }

  /* ── Left column ──────────────────────────────────────────── */
  .drop-zone {
    display: block;
    cursor: pointer;
    border: 1px dashed var(--border-default);
    aspect-ratio: 4 / 3;
    position: relative;
    transition: border-color 200ms;
  }

  .drop-zone:hover {
    border-color: var(--accent);
  }

  .drop-zone input[type='file'] {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
    width: 100%;
    height: 100%;
  }

  .drop-zone-inner {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    pointer-events: none;
  }

  .drop-icon {
    font-size: 40px;
    color: var(--fg-muted);
    font-weight: 200;
  }

  .drop-title {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-secondary);
    letter-spacing: 0.08em;
  }

  .drop-sub {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--fg-faint);
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  .preview-wrap {
    aspect-ratio: 4 / 3;
    overflow: hidden;
    background: #000;
  }

  .preview-img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    display: block;
  }

  .file-meta {
    display: flex;
    justify-content: space-between;
    margin-top: 12px;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
  }

  /* ── Error / actions ─────────────────────────────────────── */
  .error-banner {
    margin-bottom: 16px;
    padding: 12px 16px;
    border: 1px solid var(--accent);
    background: rgba(208, 160, 80, 0.08);
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--accent);
  }

  .actions {
    display: flex;
    gap: 12px;
    justify-content: flex-end;
  }

  /* ── Responsive ───────────────────────────────────────────── */
  @media (max-width: 768px) {
    .page-header {
      padding: 32px 24px 16px;
    }

    .page-title {
      font-size: 32px;
    }

    .stepper {
      font-size: 9px;
      margin-top: 20px;
    }

    .step {
      gap: 6px;
    }

    .step-check {
      margin-right: 8px;
    }

    .form-section {
      padding: 32px 24px;
    }
  }
</style>
