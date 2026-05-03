<script lang="ts">
  import PhotoTitle from './PhotoTitle.svelte';
  import Button from '../Button.svelte';

  let {
    photo
  }: {
    photo: {
      id: string;
      target?: string | null;
      original_name: string;
      last_step?: string | null;
      status: string;
    };
  } = $props();

  let stepLabel = $derived(
    photo.status === 'processing'
      ? 'STEP · PROCESSING'
      : photo.status === 'failed'
        ? 'STEP · FAILED'
        : photo.last_step === 'verify'
          ? 'STEP 02 · VERIFYING DATA'
          : photo.last_step === 'caption'
            ? 'STEP 03 · CAPTION & PUBLISH'
            : 'STEP 01 · UPLOADED'
  );

  let resumeHref = $derived(
    photo.status === 'failed'
      ? `/upload/${photo.id}/verify`
      : photo.last_step === 'caption'
        ? `/upload/${photo.id}/caption`
        : `/upload/${photo.id}/verify`
  );
</script>

<div class="draft-card">
  <div class="thumb">
    {#if photo.status === 'ready'}
      <img src={`/api/photos/${photo.id}/thumb/400`} alt="" />
    {:else}
      <div class="placeholder">{photo.status === 'failed' ? 'FAILED' : 'PROCESSING'}</div>
    {/if}
  </div>
  <div class="t-eyebrow accent">{stepLabel}</div>
  <PhotoTitle {photo} size="sm" />
  <Button variant="secondary" href={resumeHref}>Resume →</Button>
</div>

<style>
  .draft-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 16px;
    background: var(--bg-canvas);
    border: 1px dashed var(--warning, #c0a060);
  }
  .thumb {
    aspect-ratio: 4 / 3;
    background: var(--bg-canvas);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .placeholder {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.12em;
    color: var(--fg-muted);
  }
</style>
