<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import UploadResumeBanner from '$lib/components/UploadResumeBanner.svelte';
  import UploadDropzone from '$lib/components/UploadDropzone.svelte';
  import UploadFileRow from '$lib/components/UploadFileRow.svelte';
  import UploadStepper from '$lib/components/UploadStepper.svelte';
  import TierUpgradeModal from '$lib/components/TierUpgradeModal.svelte';
  import { preflight } from '$lib/upload/preflight';
  import {
    Pump,
    makeUploadRunner,
    type FileSlot,
    type SlotHandle,
    type SlotProgress
  } from '$lib/upload/pump';
  import { goto } from '$app/navigation';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
  const TIER_MAX = $derived(data.tier === 'subscriber' ? 200 * 1024 * 1024 : 50 * 1024 * 1024);

  type Slot = FileSlot & { clientId: string; thumbDataUrl?: string; progress: SlotProgress };
  let slots = $state<Slot[]>([]);
  let showUpgrade = $state(false);
  let nextId = 0;

  const handles = new Map<string, SlotHandle>();
  const pump = new Pump({
    concurrency: 3,
    runSlot: makeUploadRunner((id) => handles.get(id)),
    onCancel: (id) => {
      handles.get(id)?.abort.abort();
    }
  });

  function setProgress(clientId: string, p: SlotProgress) {
    const idx = slots.findIndex((s) => s.clientId === clientId);
    if (idx < 0) return;
    const s = slots[idx];
    if (s) s.progress = p;
  }

  async function onFiles(files: File[]) {
    for (const file of files) {
      if (file.size > TIER_MAX) {
        showUpgrade = true;
        continue;
      }
      const clientId = `c${nextId++}`;
      const slot: Slot = {
        clientId,
        name: file.name,
        size: file.size,
        mime: file.type,
        file,
        hash: '',
        // thumbDataUrl intentionally omitted — set after preflight completes.
        progress: { state: 'hashing', pct: 0 }
      };
      slots = [...slots, slot];

      preflight(file)
        .then((pre) => {
          const idx = slots.findIndex((s) => s.clientId === clientId);
          if (idx < 0) return;
          const target = slots[idx];
          if (!target) return;
          target.hash = pre.hash;
          target.thumbDataUrl = pre.thumbDataUrl;
          target.progress = { state: 'queued', pct: 0 };

          const abort = new AbortController();
          const handle: SlotHandle = {
            slot: target,
            abort,
            setProgress: (p) => setProgress(clientId, p)
          };
          handles.set(clientId, handle);
          pump.add(clientId);
        })
        .catch((err: unknown) => {
          const reason = err instanceof Error ? err.message : 'Preflight failed';
          setProgress(clientId, { state: 'failed', pct: 0, reason });
        });
    }
  }

  let readyIds = $derived(
    slots
      .filter((s) => s.progress.state === 'ready' && s.progress.photoId)
      .map((s) => s.progress.photoId!)
  );
  let allDone = $derived(
    slots.length > 0 &&
      slots.every((s) => ['ready', 'failed', 'cancelled'].includes(s.progress.state))
  );

  function cancelSlot(clientId: string) {
    const slot = slots.find((s) => s.clientId === clientId);
    if (!slot) return;

    // Confirm-on-cancel for in-flight uploads past 50%.
    if (slot.progress.state === 'uploading' && slot.progress.pct > 50) {
      if (
        !confirm(
          `Cancel upload of ${slot.name}? ${Math.round(slot.progress.pct)}% complete will be lost.`
        )
      )
        return;
    }

    pump.cancel(clientId); // triggers onCancel → abort.abort()

    // If we have a server-side photo row, ask the backend to clean it up.
    const photoId = slot.progress.photoId;
    if (photoId) {
      void fetch(`/api/uploads/${photoId}`, { method: 'DELETE', credentials: 'include' });
    }

    slots = slots.filter((s) => s.clientId !== clientId);
    handles.delete(clientId);
  }

  async function retrySlot(clientId: string) {
    const slot = slots.find((s) => s.clientId === clientId);
    if (!slot || slot.progress.state !== 'failed') return;

    // If there's a stale server-side photo row from the failed init/PUT, drop it
    // so the per-owner-hash dedup doesn't reject the retry's init.
    const oldPhotoId = slot.progress.photoId;
    if (oldPhotoId) {
      await fetch(`/api/uploads/${oldPhotoId}`, { method: 'DELETE', credentials: 'include' });
    }

    const abort = new AbortController();
    handles.set(clientId, {
      slot,
      abort,
      setProgress: (p) => setProgress(clientId, p)
    });
    slot.progress = { state: 'queued', pct: 0 };
    pump.add(clientId);
  }

  function continueToBatch() {
    if (readyIds.length === 0) return;
    if (readyIds.length === 1) {
      goto(`/upload/${readyIds[0]}/verify`);
    } else {
      goto(`/upload/batch?ids=${readyIds.join(',')}`);
    }
  }
</script>

<AppHeader active="Gallery" />

<TierUpgradeModal bind:open={showUpgrade} />

<div class="upload-page">
  <!-- Page header -->
  <section class="page-header">
    <div class="page-header-row">
      <div class="page-header-title-block">
        <div class="t-eyebrow">NEW UPLOAD · STEP 01 OF 03</div>
        <h1 class="page-title">Drop your <em>frames</em></h1>
      </div>
      <aside class="tier-rail" aria-label="Tier limits">
        <div class="tier-eyebrow t-eyebrow">
          ● {data.tier === 'subscriber' ? 'SUBSCRIBER · 200 MB / FILE' : 'FREE TIER · 50 MB / FILE'}
        </div>
        {#if data.tier !== 'subscriber'}
          <p class="tier-note">
            Subscribers upload up to 200 MB.
            <a href="/settings" class="tier-upgrade">Upgrade →</a>
          </p>
        {/if}
      </aside>
    </div>

    <UploadStepper currentStep={1} />
  </section>

  <!-- Dropzone + file list -->
  <section class="form-section">
    {#if data.recentDrafts.length}
      <UploadResumeBanner drafts={data.recentDrafts} />
    {/if}
    <UploadDropzone {onFiles} tierMax={TIER_MAX} tier={data.tier} />

    {#if slots.length}
      <div class="file-list">
        <p class="t-eyebrow list-header">● FILES ({slots.length})</p>
        {#each slots as slot (slot.clientId)}
          <UploadFileRow
            name={slot.name}
            size={slot.size}
            {...slot.thumbDataUrl !== undefined ? { thumbDataUrl: slot.thumbDataUrl } : {}}
            progress={slot.progress}
            onCancel={() => cancelSlot(slot.clientId)}
            onRetry={() => retrySlot(slot.clientId)}
          />
        {/each}
      </div>
    {/if}

    {#if allDone && readyIds.length > 0}
      <div class="continue-cta">
        <button class="btn-primary" onclick={continueToBatch}>
          Continue with {readyIds.length} frame{readyIds.length === 1 ? '' : 's'} →
        </button>
      </div>
    {/if}
  </section>
</div>

<style>
  .upload-page {
    min-height: calc(100dvh - 64px);
    background: var(--bg-base);
  }

  /* ── Page header ────────────────────────────────────────── */
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

  /* ── Page header layout ────────────────────────────────── */
  .page-header-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 32px;
  }
  .page-header-title-block {
    flex: 1 1 auto;
    min-width: 0;
  }
  .tier-rail {
    flex: 0 0 auto;
    text-align: right;
    max-width: 280px;
  }
  .tier-eyebrow {
    color: var(--accent);
  }
  .tier-note {
    margin: 8px 0 0;
    color: var(--fg-muted);
    font-size: 12px;
    line-height: 1.55;
  }
  .tier-upgrade {
    color: var(--accent);
    text-decoration: underline;
  }
  .tier-upgrade:hover {
    text-decoration: none;
  }

  /* ── Form section ───────────────────────────────────────── */
  .form-section {
    padding: 48px 64px;
  }

  /* ── File list ──────────────────────────────────────────── */
  .file-list {
    margin-top: 40px;
  }

  .list-header {
    margin-bottom: 8px;
  }

  /* ── Continue CTA ───────────────────────────────────────── */
  .continue-cta {
    margin-top: 32px;
    display: flex;
    justify-content: flex-end;
  }
  .btn-primary {
    background: var(--accent);
    color: var(--bg-base);
    padding: 12px 24px;
    border: 0;
    font-family: var(--font-mono);
    font-size: 12px;
    letter-spacing: 0.08em;
    cursor: pointer;
  }

  /* ── Responsive ─────────────────────────────────────────── */
  @media (max-width: 768px) {
    .page-header {
      padding: 32px 24px 16px;
    }

    .page-header-row {
      flex-direction: column;
      gap: 16px;
    }
    .tier-rail {
      max-width: none;
      text-align: left;
    }

    .page-title {
      font-size: 32px;
    }

    .form-section {
      padding: 32px 24px;
    }
  }
</style>
