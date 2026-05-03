<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import UploadDropzone from '$lib/components/UploadDropzone.svelte';
  import UploadFileRow from '$lib/components/UploadFileRow.svelte';
  import TierUpgradeModal from '$lib/components/TierUpgradeModal.svelte';
  import { preflight } from '$lib/upload/preflight';
  import { uploadAll, type FileSlot, type SlotProgress } from '$lib/upload/presigned';

  // Each slot gets a stable clientId so progress callbacks can find the right
  // row even when the slots array changes while a batch is in flight.
  type Slot = FileSlot & { clientId: number; thumbDataUrl?: string; progress: SlotProgress };

  let slots = $state<Slot[]>([]);
  let showUpgrade = $state(false);

  // Tier limit is hardcoded to the free-tier maximum (50 MB).
  // TODO: wire users.tier through /api/auth/me (or a dedicated endpoint) when
  // subscriber billing ships in Phase 2 so subscribers get the 200 MB limit.
  const TIER_MAX = 50 * 1024 * 1024;

  let nextId = 0;

  async function onFiles(files: File[]) {
    const newSlots: Slot[] = [];

    for (const file of files) {
      if (file.size > TIER_MAX) {
        showUpgrade = true;
        continue;
      }
      const clientId = nextId++;
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
      newSlots.push(slot);
    }

    // Append new slots so the user sees hashing feedback immediately.
    slots = [...slots, ...newSlots];

    // Run preflight on each new slot; update in place on completion.
    await Promise.all(
      newSlots.map(async (slot) => {
        try {
          const pre = await preflight(slot.file);
          const idx = slots.findIndex((s) => s.clientId === slot.clientId);
          if (idx >= 0) {
            const target = slots[idx];
            if (target) {
              target.hash = pre.hash;
              target.thumbDataUrl = pre.thumbDataUrl;
              target.progress = { state: 'queued', pct: 0 };
            }
          }
        } catch (err) {
          const reason = err instanceof Error ? err.message : 'Preflight failed';
          const idx = slots.findIndex((s) => s.clientId === slot.clientId);
          if (idx >= 0) {
            const target = slots[idx];
            if (target) {
              target.progress = { state: 'failed', pct: 0, reason };
            }
          }
        }
      })
    );

    // Only hand off slots that are truly queued (preflight done, not yet
    // submitted). Filtering on progress.state === 'queued' — rather than
    // checking hash — prevents re-uploading slots from a prior batch that are
    // already uploading or done.
    const ready = slots.filter((s) => s.progress.state === 'queued');
    if (ready.length === 0) return;

    await uploadAll(ready as FileSlot[], (i, p) => {
      const target = ready[i];
      if (!target) return;
      const slotIdx = slots.findIndex((s) => s.clientId === target.clientId);
      if (slotIdx >= 0) {
        const s = slots[slotIdx];
        if (s) s.progress = p;
      }
    });
  }
</script>

<AppHeader active="Gallery" />

<TierUpgradeModal bind:open={showUpgrade} />

<div class="upload-page">
  <!-- Page header -->
  <section class="page-header">
    <div class="t-eyebrow">NEW FRAME</div>
    <h1 class="page-title">Add a <em>frame</em> to your archive</h1>

    <!-- 3-step stepper (visual chrome — step 1 active) -->
    <div class="stepper">
      <div class="step step-active">
        <span class="step-n">01</span>
        <span>UPLOAD</span>
      </div>
      <div class="step">
        <span class="step-n">02</span>
        <span>VERIFY DATA</span>
      </div>
      <div class="step">
        <span class="step-n">03</span>
        <span>CAPTION & PUBLISH</span>
      </div>
    </div>
  </section>

  <!-- Dropzone + file list -->
  <section class="form-section">
    <UploadDropzone {onFiles} tierMax={TIER_MAX} />

    {#if slots.length}
      <div class="file-list">
        <p class="t-eyebrow list-header">● FILES ({slots.length})</p>
        {#each slots as slot (slot.clientId)}
          <UploadFileRow
            name={slot.name}
            size={slot.size}
            {...(slot.thumbDataUrl !== undefined ? { thumbDataUrl: slot.thumbDataUrl } : {})}
            progress={slot.progress}
          />
        {/each}
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

  /* ── Stepper ────────────────────────────────────────────── */
  .stepper {
    display: flex;
    margin-top: 32px;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .step {
    flex: 1;
    padding: 16px 0;
    border-top: 2px solid var(--border-default);
    display: flex;
    gap: 12px;
    align-items: center;
    color: var(--fg-muted);
  }

  .step-active {
    border-top-color: var(--accent);
    color: var(--fg-primary);
  }

  .step-n {
    color: var(--fg-faint);
  }

  .step-active .step-n {
    color: var(--accent);
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

  /* ── Responsive ─────────────────────────────────────────── */
  @media (max-width: 768px) {
    .page-header {
      padding: 32px 24px 16px;
    }

    .page-title {
      font-size: 32px;
    }

    .stepper {
      font-size: 9px;
      margin-top: 20px;
    }

    .step {
      gap: 6px;
    }

    .form-section {
      padding: 32px 24px;
    }
  }
</style>
