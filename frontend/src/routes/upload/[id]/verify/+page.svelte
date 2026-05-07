<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import Button from '$lib/components/Button.svelte';
  import UploadStepper from '$lib/components/UploadStepper.svelte';
  import VerifyPane from '$lib/components/VerifyPane.svelte';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  let isPublished = $derived(!data.photo.is_draft);
  let isProcessing = $derived(data.photo.status === 'processing');

  let pollingHandle: ReturnType<typeof setInterval> | null = null;
  $effect(() => {
    if (isProcessing && pollingHandle === null) {
      pollingHandle = setInterval(() => invalidateAll(), 2000);
    } else if (!isProcessing && pollingHandle !== null) {
      clearInterval(pollingHandle);
      pollingHandle = null;
    }
    return () => {
      if (pollingHandle !== null) {
        clearInterval(pollingHandle);
        pollingHandle = null;
      }
    };
  });
</script>

<svelte:head
  ><title>{isPublished ? 'Edit metadata' : 'Verify data'} — Astrophoto</title></svelte:head
>
<AppHeader active="Gallery" />

<div class="page">
  <div class="t-eyebrow">{isPublished ? 'EDIT METADATA' : 'NEW FRAME'}</div>
  <h1 class="title">{isPublished ? 'Edit the data.' : 'Verify the data.'}</h1>
  {#if !isPublished}<UploadStepper currentStep={2} />{/if}

  <VerifyPane photo={data.photo} initialTags={data.photo.tags ?? []} autosave={true} />

  {#if form?.error}<p class="err">{form.error}</p>{/if}

  <form method="POST" class="actions">
    {#if isPublished}
      <Button variant="primary" formaction="?/save_changes_published" type="submit"
        >Save changes</Button
      >
    {:else}
      <Button variant="ghost" formaction="?/save_draft" type="submit" disabled={isProcessing}
        >Save as draft</Button
      >
      <Button variant="primary" formaction="?/publish" type="submit" disabled={isProcessing}
        >Publish</Button
      >
    {/if}
  </form>
</div>

<style>
  .page {
    max-width: 1440px;
    margin: 0 auto;
    padding: 40px 64px 64px;
  }
  .title {
    font-family: var(--font-display);
    font-size: 44px;
    margin: 8px 0 12px;
  }
  .actions {
    display: flex;
    gap: 12px;
    justify-content: flex-end;
    margin-top: 32px;
  }
  .err {
    color: var(--danger);
  }
  @media (max-width: 768px) {
    .page {
      padding: 32px 24px;
    }
  }
</style>
