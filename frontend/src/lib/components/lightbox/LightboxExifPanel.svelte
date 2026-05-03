<script lang="ts">
  import AppreciateButton from '$lib/components/AppreciateButton.svelte';
  import type { PhotoDetail } from '$lib/api/types';

  let { photo }: { photo: PhotoDetail } = $props();

  let title = $derived(photo.target ?? 'Untitled');
</script>

<div class="exif-panel">
  <h2 class="photo-title">{title}</h2>

  {#if photo.caption}
    <p class="caption">{photo.caption}</p>
  {/if}

  <div class="appreciate-row">
    <AppreciateButton
      photoId={photo.id}
      initialCount={Number(photo.appreciation_count)}
    />
  </div>

  <dl class="exif-table">
    {#if photo.camera}
      <div class="row">
        <dt>Camera</dt>
        <dd>{photo.camera}</dd>
      </div>
    {/if}
    {#if photo.lens}
      <div class="row">
        <dt>Lens</dt>
        <dd>{photo.lens}</dd>
      </div>
    {/if}
    {#if photo.iso != null}
      <div class="row">
        <dt>ISO</dt>
        <dd>{photo.iso}</dd>
      </div>
    {/if}
    {#if photo.exposure_s != null}
      <div class="row">
        <dt>Exposure</dt>
        <dd>{photo.exposure_s}s</dd>
      </div>
    {/if}
    {#if photo.taken_at}
      <div class="row">
        <dt>Date</dt>
        <dd>{new Date(photo.taken_at).toLocaleDateString()}</dd>
      </div>
    {/if}
  </dl>
</div>

<style>
  .exif-panel {
    padding: 32px 28px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    overflow-y: auto;
    height: 100%;
    box-sizing: border-box;
  }

  .photo-title {
    font-family: var(--font-display, var(--font-ui));
    font-size: 22px;
    font-weight: 600;
    margin: 0;
    color: var(--fg-primary);
    line-height: 1.3;
  }

  .caption {
    font-size: 14px;
    line-height: 1.65;
    color: var(--fg-secondary);
    margin: 0;
  }

  .appreciate-row {
    display: flex;
    align-items: center;
  }

  .exif-table {
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .row {
    display: grid;
    grid-template-columns: 90px 1fr;
    padding: 8px 0;
    border-bottom: 1px solid var(--border-subtle);
    gap: 8px;
    align-items: baseline;
  }

  .row:first-child {
    border-top: 1px solid var(--border-subtle);
  }

  dt {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-tertiary, var(--fg-secondary));
  }

  dd {
    margin: 0;
    font-size: 13px;
    color: var(--fg-primary);
    word-break: break-word;
  }
</style>
