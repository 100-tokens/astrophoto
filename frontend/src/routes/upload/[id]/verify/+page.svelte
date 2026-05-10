<script lang="ts">
  import { goto, invalidateAll } from '$app/navigation';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import Button from '$lib/components/Button.svelte';
  import Img from '$lib/components/Img.svelte';
  import UploadStepper from '$lib/components/UploadStepper.svelte';
  import VerifyPane from '$lib/components/VerifyPane.svelte';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  let isPublished = $derived(!data.photo.is_draft);
  let isProcessing = $derived(data.photo.status === 'processing');

  let queueIds = $derived(data.queueIds ?? []);
  let queueIndex = $derived(data.queueIndex ?? -1);
  let inBatch = $derived(queueIds.length > 1 && queueIndex >= 0);
  let frameLabel = $derived(inBatch ? `FRAME ${queueIndex + 1} OF ${queueIds.length}` : '');
  let prevId = $derived(inBatch && queueIndex > 0 ? queueIds[queueIndex - 1] : null);
  let nextId = $derived(
    inBatch && queueIndex < queueIds.length - 1 ? queueIds[queueIndex + 1] : null
  );
  let qs = $derived(`?ids=${queueIds.join(',')}`);

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

  // Keyboard nav between frames in batch context. Plain ←/→ jumps when
  // no input is focused, so it feels like browsing a slideshow rather
  // than editing one frame at a time.
  $effect(() => {
    if (!inBatch) return;
    function isEditable(t: EventTarget | null): boolean {
      if (!(t instanceof HTMLElement)) return false;
      const tag = t.tagName;
      return (
        tag === 'INPUT' ||
        tag === 'TEXTAREA' ||
        tag === 'SELECT' ||
        t.isContentEditable === true
      );
    }
    function onKey(e: KeyboardEvent) {
      if (isEditable(e.target)) return;
      if (e.key === 'ArrowLeft' && prevId) {
        e.preventDefault();
        void goto(`/upload/${prevId}/verify${qs}`);
      } else if (e.key === 'ArrowRight' && nextId) {
        e.preventDefault();
        void goto(`/upload/${nextId}/verify${qs}`);
      }
    }
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
</script>

<svelte:head
  ><title>{isPublished ? 'Edit metadata' : 'Verify data'} — Astrophoto</title></svelte:head
>
<AppHeader active="Gallery" />

<div class="page">
  <div class="t-eyebrow">
    {#if isPublished}
      EDIT METADATA
    {:else}
      STEP 02 OF 03 · VERIFY{#if inBatch} · {frameLabel}{/if}
    {/if}
  </div>
  <h1 class="title">
    {#if isPublished}
      Edit the data.
    {:else}
      What's <em>in</em> this frame?
    {/if}
  </h1>
  {#if !isPublished}<UploadStepper currentStep={2} />{/if}

  {#if inBatch}
    <nav class="queue-nav" aria-label="Batch navigation">
      <span class="queue-eyebrow t-eyebrow">QUEUE · FRAMES TO VERIFY</span>
      <div class="queue-thumbs">
        {#each queueIds as id, i}
          <a
            class="queue-thumb"
            class:queue-thumb--current={i === queueIndex}
            href={`/upload/${id}/verify${qs}`}
            aria-current={i === queueIndex ? 'step' : undefined}
            aria-label={`Frame ${i + 1} of ${queueIds.length}`}
          >
            <Img photoId={id} alt="" w={120} />
          </a>
        {/each}
      </div>
      <p class="queue-hint t-meta">
        ← / → BETWEEN FRAMES{#if nextId} · → JUMPS TO NEXT{/if}
      </p>
    </nav>
  {/if}

  {#key data.photo.id}
    <VerifyPane photo={data.photo} initialTags={data.photo.tags ?? []} autosave={true} />
  {/key}

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
      {#if inBatch && nextId}
        <Button variant="ghost" href={`/upload/${nextId}/verify${qs}`}>Skip frame →</Button>
      {/if}
      <Button variant="primary" formaction="?/publish" type="submit" disabled={isProcessing}>
        {#if inBatch}Publish · {queueIndex + 1} of {queueIds.length}{:else}Publish{/if}
      </Button>
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
  .queue-nav {
    margin: 24px 0 32px;
    padding: 16px;
    border: 1px solid var(--border-default);
    background: var(--bg-base, transparent);
  }
  .queue-eyebrow {
    margin: 0;
  }
  .queue-thumbs {
    display: flex;
    gap: 8px;
    margin-top: 12px;
    flex-wrap: wrap;
  }
  .queue-thumb {
    position: relative;
    width: 80px;
    height: 60px;
    overflow: hidden;
    border: 1px solid var(--border-default);
    flex-shrink: 0;
  }
  .queue-thumb :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .queue-thumb--current {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .queue-hint {
    margin: 12px 0 0;
    color: var(--fg-muted);
    letter-spacing: 0.06em;
  }
  @media (max-width: 768px) {
    .page {
      padding: 32px 24px;
    }
  }
</style>
