<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import UploadDropzone from '$lib/components/UploadDropzone.svelte';
  import UploadFileRow from '$lib/components/UploadFileRow.svelte';
  import UploadStepper from '$lib/components/UploadStepper.svelte';
  import TierUpgradeModal from '$lib/components/TierUpgradeModal.svelte';
  import { preflight } from '$lib/upload/preflight';
  import { Pump, makeUploadRunner, type FileSlot, type SlotHandle, type SlotProgress } from '$lib/upload/pump';
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
        progress: { state: 'hashing', pct: 0 },
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
            setProgress: (p) => setProgress(clientId, p),
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
    slots.filter((s) => s.progress.state === 'ready' && s.progress.photoId).map((s) => s.progress.photoId!)
  );
  let allDone = $derived(
    slots.length > 0 && slots.every((s) => ['ready', 'failed', 'cancelled'].includes(s.progress.state))
  );

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
    <div class="t-eyebrow">NEW FRAME</div>
    <h1 class="page-title">Add a <em>frame</em> to your archive</h1>

    <UploadStepper currentStep={1} />
  </section>

  <!-- Dropzone + file list -->
  <section class="form-section">
    {#if data.recentDrafts.length}
      <!-- TODO: resume banner in T9 -->
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

  /* ── Form section ───────────────────────────────────────── */
  .form-section {
    padding: 48px 64px;
    max-width: 800px;
  }

  /* ── File list ──────────────────────────────────────────── */
  .file-list {
    margin-top: 40px;
  }

  .list-header {
    margin-bottom: 8px;
  }

  /* ── Continue CTA ───────────────────────────────────────── */
  .continue-cta { margin-top: 32px; display: flex; justify-content: flex-end; }
  .btn-primary { background: var(--accent); color: var(--bg-base); padding: 12px 24px; border: 0; font-family: var(--font-mono); font-size: 12px; letter-spacing: 0.08em; cursor: pointer; }

  /* ── Responsive ─────────────────────────────────────────── */
  @media (max-width: 768px) {
    .page-header {
      padding: 32px 24px 16px;
    }

    .page-title {
      font-size: 32px;
    }

    .form-section {
      padding: 32px 24px;
    }
  }
</style>
