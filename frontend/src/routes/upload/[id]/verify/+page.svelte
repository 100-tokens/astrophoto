<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  let polling = $state<number | null>(null);

  let isPublished = $derived(!data.photo.is_draft);
  let isProcessing = $derived(data.photo.status === 'processing');
  let isFailed = $derived(data.photo.status === 'failed');

  $effect(() => {
    if (isProcessing && polling === null) {
      polling = window.setInterval(() => invalidateAll(), 2000);
    }
    if (!isProcessing && polling !== null) {
      clearInterval(polling);
      polling = null;
    }
    return () => {
      if (polling !== null) clearInterval(polling);
    };
  });
</script>

<svelte:head><title>Verify data — Astrophoto</title></svelte:head>
<AppHeader active="Gallery" />

<div class="verify-page">
  <div class="t-eyebrow">{isPublished ? 'EDIT METADATA' : 'NEW FRAME · STEP 02'}</div>
  <h1 class="title">Verify the <em>data</em>.</h1>

  {#if isFailed}
    <div class="panel-failed">
      <div class="t-eyebrow danger">
        ● UPLOAD FAILED · {data.photo.pipeline_error ?? 'unknown error'}
      </div>
      <div class="actions">
        <form method="POST" action="?/save_draft">
          <Button variant="ghost" type="submit">Discard</Button>
        </form>
        <Button variant="primary" href="/upload">Retry upload</Button>
      </div>
    </div>
  {:else}
    <form
      method="POST"
      action={isPublished ? '?/save_changes_published' : '?/save_continue'}
      class="metadata-form"
    >
      <fieldset disabled={isProcessing}>
        <div class="grid">
          <label>
            <span class="t-label">TARGET</span>
            <Input name="target" value={data.photo.target ?? ''} placeholder="M31, NGC 7000…" />
          </label>
          <label>
            <span class="t-label">CAMERA</span>
            <Input name="camera" value={data.photo.camera ?? ''} />
          </label>
          <label>
            <span class="t-label">LENS</span>
            <Input name="lens" value={data.photo.lens ?? ''} />
          </label>
          <label>
            <span class="t-label">ISO</span>
            <Input type="number" name="iso" value={data.photo.iso?.toString() ?? ''} />
          </label>
          <label>
            <span class="t-label">EXPOSURE (S)</span>
            <Input
              type="number"
              step="0.01"
              name="exposure_s"
              value={data.photo.exposure_s?.toString() ?? ''}
            />
          </label>
          <label>
            <span class="t-label">FOCAL (MM)</span>
            <Input type="number" name="focal_mm" value={data.photo.focal_mm?.toString() ?? ''} />
          </label>
        </div>
      </fieldset>

      {#if isProcessing}
        <p class="t-meta">● PROCESSING THUMBNAILS — polling every 2 s</p>
      {/if}
      {#if form?.error}
        <p class="t-meta form-error">{form.error}</p>
      {/if}

      <div class="actions">
        {#if isPublished}
          <Button variant="ghost" href="/upload/{data.photo.id}/caption">Edit caption →</Button>
          <Button variant="primary" type="submit" disabled={isProcessing}>Save changes</Button>
        {:else}
          <Button
            variant="ghost"
            type="submit"
            formaction="?/save_draft"
            disabled={isProcessing}
          >Save as draft</Button>
          <Button variant="primary" type="submit" disabled={isProcessing}>Continue →</Button>
        {/if}
      </div>
    </form>
  {/if}
</div>

<style>
  .verify-page {
    padding: 40px 64px;
    max-width: 960px;
    margin: 0 auto;
  }
  .title {
    font-family: var(--font-display);
    font-size: 44px;
    margin: 8px 0 32px;
  }
  .title em {
    font-style: italic;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px 24px;
  }
  .grid label {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .actions {
    display: flex;
    gap: 12px;
    justify-content: flex-end;
    margin-top: 32px;
  }
  .panel-failed {
    padding: 24px;
    border: 1px solid var(--danger);
    margin-top: 32px;
  }
  .danger {
    color: var(--danger);
  }
  .form-error {
    color: var(--danger);
  }
  @media (max-width: 768px) {
    .verify-page {
      padding: 32px 24px;
    }
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
