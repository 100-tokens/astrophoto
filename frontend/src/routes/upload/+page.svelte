<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import UploadResumeBanner from '$lib/components/UploadResumeBanner.svelte';
  import UploadDropzone from '$lib/components/UploadDropzone.svelte';
  import UploadFileRow from '$lib/components/UploadFileRow.svelte';
  import UploadStepper from '$lib/components/UploadStepper.svelte';
  import TierUpgradeModal from '$lib/components/TierUpgradeModal.svelte';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
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
  // Design caps the queue at 12 files at once (showcase-p1.jsx:108).
  // Server enforces the same limit per upload-init batch.
  const MAX_QUEUE = 12;

  type Slot = FileSlot & { clientId: string; thumbDataUrl?: string; progress: SlotProgress };
  let slots = $state<Slot[]>([]);
  let showUpgrade = $state(false);
  let queueCapWarning = $state<string | null>(null);
  let nextId = 0;

  function fmtBytes(n: number): string {
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
    if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
    return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }
  let storagePct = $derived.by(() => {
    if (!data.storage) return 0;
    const quota = Number(data.storage.quota_bytes);
    if (quota === 0) return 0;
    return Math.round((Number(data.storage.used_bytes) / quota) * 100);
  });

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
    // 12-file cap matches both the design and the upload-init server check.
    // Truncate silently rather than reject the whole batch so a 13-file drag
    // still gets the first 12 onto the queue.
    queueCapWarning = null;
    const room = MAX_QUEUE - slots.length;
    if (room <= 0) {
      queueCapWarning = `Queue is full — ${MAX_QUEUE} files at a time. Verify or clear what's already here first.`;
      return;
    }
    if (files.length > room) {
      queueCapWarning = `Queue caps at ${MAX_QUEUE} files. Added the first ${room}, skipped the rest.`;
      files = files.slice(0, room);
    }
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
          // XISF: preflight returns an empty thumb (no browser decoder).
          // Leave thumbDataUrl undefined so UploadFileRow falls back to
          // its generic icon instead of rendering an empty <img>.
          if (pre.thumbDataUrl) target.thumbDataUrl = pre.thumbDataUrl;
          // Preflight resolves the wire mime (browsers report "" for
          // `.xisf`; preflight maps it to `application/x-xisf`).
          target.mime = pre.mime;
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
  // Counts driving the queue header. Inflight = anything still moving.
  let queueCounts = $derived.by(() => {
    let ready = 0;
    let inflight = 0;
    let blocked = 0;
    for (const s of slots) {
      if (s.progress.state === 'ready') ready++;
      else if (s.progress.state === 'failed') blocked++;
      else if (s.progress.state !== 'cancelled') inflight++;
    }
    return { ready, inflight, blocked };
  });
  let allDone = $derived(
    slots.length > 0 &&
      slots.every((s) => ['ready', 'failed', 'cancelled'].includes(s.progress.state))
  );

  let clearQueueOpen = $state(false);
  let cancelSlotOpen = $state(false);
  let cancelSlotPending = $state<{ clientId: string; name: string; pct: number } | null>(null);

  function clearQueue() {
    if (slots.some((s) => s.progress.state === 'uploading')) {
      clearQueueOpen = true;
      return;
    }
    performClearQueue();
  }

  function performClearQueue() {
    clearQueueOpen = false;
    for (const s of slots) {
      pump.cancel(s.clientId);
      const photoId = s.progress.photoId;
      if (photoId) {
        void fetch(`/api/uploads/${photoId}`, { method: 'DELETE', credentials: 'include' });
      }
    }
    handles.clear();
    slots = [];
  }

  function cancelSlot(clientId: string) {
    const slot = slots.find((s) => s.clientId === clientId);
    if (!slot) return;

    // Confirm-on-cancel for in-flight uploads past 50%.
    if (slot.progress.state === 'uploading' && slot.progress.pct > 50) {
      cancelSlotPending = {
        clientId,
        name: slot.name,
        pct: slot.progress.pct
      };
      cancelSlotOpen = true;
      return;
    }
    performCancelSlot(clientId);
  }

  function performCancelSlot(clientId: string) {
    const slot = slots.find((s) => s.clientId === clientId);
    if (!slot) return;

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
      // Land on the first frame with the queue context in the URL —
      // the verify page renders a thumbs strip + ←/→ keyboard nav and
      // a Skip frame → / Publish · N of M action row from there. The
      // older /upload/batch landing (apply common fields) is reachable
      // from the verify page if we ever add a "Edit all at once" link.
      goto(`/upload/${readyIds[0]}/verify?ids=${readyIds.join(',')}`);
    }
  }
</script>

<svelte:head>
  <title>Upload — Astrophoto</title>
  <meta name="robots" content="noindex, nofollow" />
</svelte:head>

<AppHeader active="Gallery" />

<TierUpgradeModal bind:open={showUpgrade} />

<ConfirmDialog
  bind:open={clearQueueOpen}
  title="Cancel uploads"
  message="Cancel all in-flight uploads and clear the queue?"
  confirmLabel="Cancel uploads"
  tone="danger"
  onconfirm={performClearQueue}
/>

<ConfirmDialog
  bind:open={cancelSlotOpen}
  title="Cancel upload"
  message={cancelSlotPending
    ? `Cancel upload of ${cancelSlotPending.name}? ${Math.round(cancelSlotPending.pct)}% complete will be lost.`
    : ''}
  confirmLabel="Cancel upload"
  tone="danger"
  onconfirm={() => {
    if (cancelSlotPending) performCancelSlot(cancelSlotPending.clientId);
    cancelSlotOpen = false;
    cancelSlotPending = null;
  }}
/>

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
            <button type="button" class="tier-upgrade" onclick={() => (showUpgrade = true)}
              >Upgrade →</button
            >
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

    {#if queueCapWarning}
      <p class="cap-warning t-meta">⚠ {queueCapWarning}</p>
    {/if}

    {#if slots.length}
      <div class="file-list">
        <header class="queue-header">
          <p class="t-eyebrow">
            ● FILES · {slots.length} · {queueCounts.ready} ready · {queueCounts.inflight} uploading
            {#if queueCounts.blocked > 0}· {queueCounts.blocked} blocked{/if}
          </p>
          <button type="button" class="chip" onclick={clearQueue}>Clear queue</button>
        </header>
        <div class="rows">
          {#each slots as slot (slot.clientId)}
            <UploadFileRow
              name={slot.name}
              size={slot.size}
              hash={slot.hash}
              {...slot.thumbDataUrl !== undefined ? { thumbDataUrl: slot.thumbDataUrl } : {}}
              progress={slot.progress}
              onCancel={() => cancelSlot(slot.clientId)}
              onRetry={() => retrySlot(slot.clientId)}
            />
          {/each}
        </div>
      </div>

      <footer class="queue-footer">
        {#if data.storage}
          <span class="storage-line t-meta">
            STORAGE · {fmtBytes(Number(data.storage.used_bytes))} / {fmtBytes(
              Number(data.storage.quota_bytes)
            )} USED · {storagePct} % &nbsp;·&nbsp; CHECKSUM DEDUP IS PER-OWNER
          </span>
        {/if}
        <div class="footer-actions">
          <a href="/drafts" class="btn-ghost">Save & finish later</a>
          <button
            class="btn-primary"
            onclick={continueToBatch}
            disabled={readyIds.length === 0}
            title={readyIds.length === 0
              ? 'Wait for at least one upload to finish'
              : `Verify ${readyIds.length} ready frame${readyIds.length === 1 ? '' : 's'}`}
          >
            {readyIds.length > 0
              ? `Verify ${readyIds.length} ready frame${readyIds.length === 1 ? '' : 's'} →`
              : 'Verify ready frames →'}
          </button>
        </div>
      </footer>
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
    background: transparent;
    border: 0;
    padding: 0;
    color: var(--accent);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
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
  .queue-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }
  .queue-header .t-eyebrow {
    margin: 0;
  }
  .chip {
    padding: 4px 10px;
    background: transparent;
    border: 1px solid var(--border-default);
    color: var(--fg-secondary);
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.06em;
    cursor: pointer;
  }
  .chip:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
  .rows {
    border: 1px solid var(--border-subtle);
  }

  /* ── Footer ─────────────────────────────────────────────── */
  .queue-footer {
    margin-top: 24px;
    padding-top: 24px;
    border-top: 1px dashed var(--border-default);
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }
  .footer-actions {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-left: auto;
  }
  .storage-line {
    color: var(--fg-muted);
  }
  .cap-warning {
    margin: 16px 0 0;
    color: var(--warning, #c98a3a);
  }
  .btn-ghost {
    background: transparent;
    border: 0;
    color: var(--fg-secondary);
    text-decoration: none;
    padding: 12px 0;
    font-family: var(--font-mono);
    font-size: 12px;
    letter-spacing: 0.08em;
    cursor: pointer;
  }
  .btn-ghost:hover {
    color: var(--accent);
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
  .btn-primary:disabled {
    opacity: 0.45;
    cursor: not-allowed;
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
