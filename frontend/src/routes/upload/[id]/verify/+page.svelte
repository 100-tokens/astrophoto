<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import Button from '$lib/components/Button.svelte';
  import CategorySegmented from '$lib/components/CategorySegmented.svelte';
  import EquipmentAutocomplete from '$lib/components/EquipmentAutocomplete.svelte';
  import Input from '$lib/components/Input.svelte';
  import TagInput from '$lib/components/TagInput.svelte';
  import TargetPicker from '$lib/components/TargetPicker.svelte';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  let polling = $state<number | null>(null);

  // The generated PhotoDetail type doesn't yet include the showcase fields
  // (category, scope, mount, filters, guiding). Cast once to access them.
  // TODO: re-run `just types` once backend exports these fields via ts-rs.
  type ShowcasePhoto = typeof data.photo & {
    category?: string | null;
    scope?: string | null;
    mount?: string | null;
    filters?: string | null;
    guiding?: string | null;
  };
  const sp: ShowcasePhoto = data.photo as ShowcasePhoto;

  let target   = $state<string>(sp.target   ?? '');
  let camera   = $state<string>(sp.camera   ?? '');
  let tags     = $state<string[]>([]);
  let category = $state<string>(sp.category ?? 'other');
  let scope    = $state<string>(sp.scope    ?? '');
  let mount    = $state<string>(sp.mount    ?? '');
  let filters  = $state<string>(sp.filters  ?? '');
  let guiding  = $state<string>(sp.guiding  ?? '');
  // TODO(P2): load existing tags from photo_tags join in the load function.

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
        <!-- Row 1: target + category (full-width each) -->
        <div class="field-full">
          <TargetPicker bind:value={target} />
        </div>

        <div class="field-full">
          <CategorySegmented bind:value={category} />
        </div>

        <!-- Row 2: numeric EXIF fields in 2-col grid -->
        <div class="grid">
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

        <!-- Row 3: equipment pickers in 2-col grid -->
        <div class="grid equipment-grid">
          <div class="field">
            <EquipmentAutocomplete name="camera" kind="camera" bind:value={camera} />
          </div>
          <div class="field">
            <EquipmentAutocomplete name="scope" kind="telescope" bind:value={scope} />
          </div>
          <div class="field">
            <EquipmentAutocomplete name="mount" kind="mount" bind:value={mount} />
          </div>
          <div class="field">
            <EquipmentAutocomplete name="filters" kind="filter" bind:value={filters} />
          </div>
          <div class="field">
            <EquipmentAutocomplete name="guiding" kind="guiding" bind:value={guiding} />
          </div>
        </div>

        <!-- Row 4: tags (full width) -->
        <div class="field-full">
          <TagInput bind:value={tags} />
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
          <Button variant="ghost" type="submit" formaction="?/save_draft" disabled={isProcessing}
            >Save as draft</Button
          >
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
  .field-full {
    margin-bottom: 16px;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px 24px;
    margin-bottom: 16px;
  }
  .grid label {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .equipment-grid .field {
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
