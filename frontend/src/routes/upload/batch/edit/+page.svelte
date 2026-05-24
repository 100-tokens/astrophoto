<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import BatchRibbon from '$lib/components/BatchRibbon.svelte';
  import Button from '$lib/components/Button.svelte';
  import UploadStepper from '$lib/components/UploadStepper.svelte';
  import VerifyPane from '$lib/components/VerifyPane.svelte';
  import { goto, invalidateAll } from '$app/navigation';
  import { page } from '$app/stores';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();

  let selectedPhoto = $derived(data.photos.find((p) => p.id === data.selected) ?? data.photos[0]);

  // Poll while any photo is still being processed.
  let anyProcessing = $derived(data.photos.some((p) => p.status === 'processing'));
  let pollingHandle: ReturnType<typeof setInterval> | null = null;
  $effect(() => {
    if (anyProcessing && pollingHandle === null) {
      pollingHandle = setInterval(() => invalidateAll(), 2000);
    } else if (!anyProcessing && pollingHandle !== null) {
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

  function selectPhoto(id: string) {
    const params = new URLSearchParams($page.url.searchParams);
    params.set('selected', id);
    goto(`/upload/batch/edit?${params}`, { replaceState: false, noScroll: true, keepFocus: true });
  }
</script>

<svelte:head><title>Verify frames — Astrophoto</title></svelte:head>
<AppHeader active="Gallery" />

<main>
  <div class="page">
    <div class="t-eyebrow">NEW FRAMES</div>
    <h1 class="title">Verify the <em>data</em>.</h1>
    <UploadStepper currentStep={2} />

    <BatchRibbon photos={data.photos} selectedId={data.selected} onSelect={selectPhoto} />

    {#key data.selected}
      {#if selectedPhoto}
        <VerifyPane photo={selectedPhoto} initialTags={selectedPhoto.tags} autosave={true} />
      {/if}
    {/key}

    {#if form?.error}
      <p class="err">{form.error}</p>
    {/if}

    <form method="POST" action={`?/publish_all&ids=${data.ids.join(',')}`} class="footer">
      <Button variant="ghost" href={`/upload/batch?ids=${data.ids.join(',')}`}
        >← Back to apply-to-all</Button
      >
      <Button variant="primary" type="submit">Publish all</Button>
    </form>
  </div>
</main>

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
  .footer {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-top: 32px;
    padding-top: 24px;
    border-top: 1px solid var(--border-subtle);
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
