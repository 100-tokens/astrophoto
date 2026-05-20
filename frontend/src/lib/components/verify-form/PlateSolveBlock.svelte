<script lang="ts">
  import type { PlatesolveStatus } from '$lib/api/PlatesolveStatus';

  // PlateSolveBlock — pure presentation wrapper around the existing
  // plate-solve state machine. The page owns psStatus / psFile / submit
  // callbacks; this component renders the framed block with status pill,
  // drop zone, and Re-calibrate button.
  //
  // The three variants (idle / solved / failed) all share the same chrome:
  //   - bordered container with accent-dim left stripe
  //   - drop zone + primary CTA in a 1fr/auto grid
  // The status pill above the controls is hidden in "idle" so the block
  // doesn't claim a state it has no data for.

  interface Props {
    status: PlatesolveStatus | null;
    file: File | null;
    submitting: boolean;
    /** Localized error from the input (e.g. file too large) — shown below. */
    localError: string | null;
    /** Server-side limit, mirrored from backend/src/photos/platesolve.rs. */
    maxBytes: number;
    onPick: (e: Event) => void;
    onCalibrate: () => void;
  }

  let { status, file, submitting, localError, maxBytes, onPick, onCalibrate }: Props = $props();

  let state = $derived(status?.state ?? 'idle');
  let isSolving = $derived(state === 'solving');
  let isSolved = $derived(state === 'solved');
  let isFailed = $derived(state === 'failed');

  function formatRaSexa(deg: number | null | undefined): string {
    if (deg == null || !Number.isFinite(deg)) return '—';
    // Convert decimal degrees to RA hours:minutes:seconds (deg/15).
    const hours = deg / 15;
    const h = Math.floor(hours);
    const m = Math.floor((hours - h) * 60);
    const s = ((hours - h) * 60 - m) * 60;
    return `${String(h).padStart(2, '0')}h ${String(m).padStart(2, '0')}m ${s.toFixed(1)}s`;
  }
  function formatDecSexa(deg: number | null | undefined): string {
    if (deg == null || !Number.isFinite(deg)) return '—';
    const sign = deg >= 0 ? '+' : '-';
    const abs = Math.abs(deg);
    const d = Math.floor(abs);
    const m = Math.floor((abs - d) * 60);
    const s = ((abs - d) * 60 - m) * 60;
    return `${sign}${String(d).padStart(2, '0')}° ${String(m).padStart(2, '0')}′ ${s.toFixed(0)}″`;
  }

  let maxMb = $derived(Math.floor(maxBytes / (1024 * 1024)));
</script>

<section class="ps-block" class:ps-block--failed={isFailed} aria-live="polite">
  <header class="ps-head">
    <div class="t-label">OPTIONAL · PLATE SOLVE</div>
    <span class="t-meta ps-source">ASTROMETRY.NET LOCAL</span>
  </header>
  <p class="ps-body">
    {#if isFailed && status?.error}
      Solve failed — re-upload an XISF / FITS master or try again. The server reported: {status.error}
    {:else}
      We ran the XISF header through a local solve. Re-upload a different frame or recalibrate if
      the coordinates feel off.
    {/if}
  </p>

  {#if isSolving}
    <div class="ps-pill ps-pill--solving">
      <span class="dot">●</span>
      <span class="key">SOLVING</span>
      <span class="sep">·</span>
      <span class="val">polling every 2 s</span>
    </div>
  {:else if isSolved && status}
    <div class="ps-pill">
      <span class="dot">●</span>
      <span class="key">SOLVED</span>
      {#if status.raDeg != null}
        <span class="sep">·</span>
        <span class="lbl">RA</span>
        <span class="v">{formatRaSexa(status.raDeg)}</span>
      {/if}
      {#if status.decDeg != null}
        <span class="sep">·</span>
        <span class="lbl">DEC</span>
        <span class="v">{formatDecSexa(status.decDeg)}</span>
      {/if}
      {#if status.pixelScaleArcsec != null}
        <span class="sep">·</span>
        <span class="v">{status.pixelScaleArcsec.toFixed(3)}″/px</span>
      {/if}
      {#if status.rotationDeg != null}
        <span class="sep">·</span>
        <span class="lbl">rot</span>
        <span class="v">{status.rotationDeg.toFixed(2)}°</span>
      {/if}
      {#if status.rmsArcsec != null}
        <span class="sep">·</span>
        <span class="lbl">RMS</span>
        <span class="v">{status.rmsArcsec.toFixed(3)}″</span>
      {/if}
      {#if status.matchedCount != null && status.detectedCount != null}
        <span class="sep">·</span>
        <span class="lbl">matched</span>
        <span class="v">{status.matchedCount}/{status.detectedCount}</span>
      {/if}
    </div>
  {:else if isFailed}
    <div class="ps-pill ps-pill--failed">
      <span class="dot">×</span>
      <span class="key">COULD NOT SOLVE</span>
    </div>
  {/if}

  <div class="ps-controls">
    <label class="ps-drop">
      <svg
        width="22"
        height="22"
        viewBox="0 0 24 24"
        fill="none"
        stroke="var(--fg-muted)"
        stroke-width="1.4"
        aria-hidden="true"
      >
        <path d="M12 16V4M12 4l-5 5M12 4l5 5" />
        <path d="M4 16v3a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-3" />
      </svg>
      <div class="ps-drop-text">
        <span class="ps-drop-primary">
          {file ? file.name : 'Drop a new XISF / FITS to re-solve'}
        </span>
        <span class="t-meta">
          {file
            ? `${(file.size / (1024 * 1024)).toFixed(1)} MB · ready to upload`
            : `or click to browse · max ${maxMb} MB`}
        </span>
      </div>
      <input
        class="vh"
        type="file"
        accept=".xisf,.fits,application/x-xisf"
        disabled={isSolving || submitting}
        onchange={onPick}
      />
    </label>
    <button
      type="button"
      class="btn btn-secondary btn-lg ps-cta"
      disabled={!file || submitting || isSolving}
      onclick={onCalibrate}
    >
      {#if submitting}
        Uploading…
      {:else if isSolving}
        Solving…
      {:else if isSolved}
        ↻ Re-calibrate
      {:else if isFailed}
        ↻ Retry
      {:else}
        ↻ Plate-solve this frame
      {/if}
    </button>
  </div>
  {#if localError}
    <p class="ps-error t-meta">{localError}</p>
  {/if}
</section>

<style>
  .ps-block {
    border: 1px solid var(--border-subtle);
    border-left: 2px solid var(--accent-dim);
    padding: 24px 28px;
    background: rgba(232, 164, 58, 0.025);
    border-radius: var(--r-sm);
  }
  .ps-block--failed {
    border-left-color: var(--danger);
  }
  .ps-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 4px;
  }
  .ps-source {
    color: var(--fg-faint);
  }
  .ps-body {
    color: var(--fg-secondary);
    font-size: 13px;
    margin: 0 0 18px;
    max-width: 540px;
    line-height: 1.5;
  }
  .ps-pill {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--bg-base);
    border: 1px solid var(--accent-dim);
    border-radius: var(--r-sm);
    margin-bottom: 20px;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.04em;
  }
  .ps-pill--failed {
    border-color: var(--border-danger);
  }
  .ps-pill--failed .dot,
  .ps-pill--failed .key {
    color: var(--danger);
  }
  .ps-pill .dot {
    color: var(--accent);
    font-weight: 600;
  }
  .ps-pill .key {
    color: var(--accent);
    font-weight: 600;
  }
  .ps-pill--solving .key,
  .ps-pill--solving .dot {
    color: var(--accent);
  }
  .ps-pill .sep {
    color: var(--fg-faint);
  }
  .ps-pill .lbl {
    color: var(--fg-secondary);
  }
  .ps-pill .v {
    color: var(--fg-primary);
  }
  .ps-pill .val {
    color: var(--fg-secondary);
  }
  .ps-controls {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 16px;
    align-items: stretch;
  }
  .ps-drop {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 18px;
    border: 1px dashed var(--border-default);
    background: var(--bg-base);
    border-radius: var(--r-sm);
    cursor: pointer;
    transition: border-color 0.15s var(--ease-out);
  }
  .ps-drop:hover {
    border-color: var(--accent);
  }
  .ps-drop:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(232, 164, 58, 0.12);
  }
  .ps-drop-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .ps-drop-primary {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-secondary);
    letter-spacing: 0.04em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ps-cta {
    padding-left: 18px;
    padding-right: 18px;
  }
  .ps-error {
    color: var(--danger);
    margin: 12px 0 0;
  }
  .vh {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
  @media (max-width: 640px) {
    .ps-controls {
      grid-template-columns: 1fr;
    }
    .ps-block {
      padding: 20px;
    }
  }
</style>
