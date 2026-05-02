<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import Button from '$lib/components/Button.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  let isPublished = $derived(!data.photo.is_draft);
</script>

<svelte:head><title>Caption — Astrophoto</title></svelte:head>
<AppHeader active="Gallery" />

<div class="caption-page">
  <div class="t-eyebrow">{isPublished ? 'EDIT CAPTION' : 'NEW FRAME · STEP 03'}</div>
  <h1 class="title">{isPublished ? 'Edit the caption.' : 'Add a caption.'}</h1>

  <div class="recap">
    <div><span class="t-label">TARGET</span> {data.photo.target ?? '—'}</div>
    <div><span class="t-label">CAMERA</span> {data.photo.camera ?? '—'}</div>
    <div><span class="t-label">EXPOSURE</span> {data.photo.exposure_s ?? '—'} s</div>
  </div>

  <form method="POST" action={isPublished ? '?/save_changes' : '?/publish'} class="caption-form">
    <Textarea
      name="caption"
      rows={8}
      value={data.photo.caption ?? ''}
      placeholder="Describe the conditions, processing, equipment used…"
    />
    {#if form?.error}<p class="t-meta form-error">{form.error}</p>{/if}
    <div class="actions">
      {#if !isPublished}
        <Button variant="ghost" type="submit" formaction="?/save_draft">Save as draft</Button>
      {/if}
      <Button variant="primary" type="submit">{isPublished ? 'Save changes' : 'Publish'}</Button>
    </div>
  </form>
</div>

<style>
  .caption-page {
    padding: 40px 64px;
    max-width: 720px;
    margin: 0 auto;
  }
  .title {
    font-family: var(--font-display);
    font-size: 44px;
    margin: 8px 0 32px;
  }
  .recap {
    display: flex;
    gap: 32px;
    padding: 16px;
    background: var(--bg-surface);
    margin-bottom: 24px;
  }
  .recap > div {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .actions {
    display: flex;
    gap: 12px;
    justify-content: flex-end;
    margin-top: 24px;
  }
  .form-error {
    color: var(--danger);
  }
  @media (max-width: 768px) {
    .caption-page {
      padding: 32px 24px;
    }
    .recap {
      flex-direction: column;
      gap: 8px;
    }
  }
</style>
