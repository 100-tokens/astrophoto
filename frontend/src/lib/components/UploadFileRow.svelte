<script lang="ts">
  import type { SlotProgress } from '$lib/upload/pump';

  interface Props {
    name: string;
    size: number;
    hash?: string;
    thumbDataUrl?: string;
    progress: SlotProgress;
    onRetry?: () => void;
    onCancel?: () => void;
  }

  let { name, size, hash, thumbDataUrl, progress, onRetry, onCancel }: Props = $props();

  const sizeMb = $derived((size / 1024 / 1024).toFixed(1));
  const cancellable = $derived(['hashing', 'queued', 'uploading'].includes(progress.state));

  // Short SHA-256 chip — lets repeat uploaders eyeball a checksum mismatch
  // without staring at a 64-char hex blob. Empty until preflight finishes.
  const shortHash = $derived(hash && hash.length >= 8 ? `${hash.slice(0, 4)}…${hash.slice(-4)}` : '');

  // State pill copy + colour. Keeps the inflight states accent-warm and the
  // terminal failed/cancelled states danger-coloured per design.
  type Pill = { label: string; tone: 'accent' | 'info' | 'success' | 'danger' | 'muted' };
  const pill = $derived<Pill>(
    progress.state === 'ready'
      ? { label: '✓ READY', tone: 'success' }
      : progress.state === 'uploading'
        ? { label: '↑ UPLOADING', tone: 'accent' }
        : progress.state === 'hashing'
          ? { label: '◐ HASHING', tone: 'info' }
          : progress.state === 'queued'
            ? { label: '◯ QUEUED', tone: 'muted' }
            : progress.state === 'finalizing'
              ? { label: '⋯ FINALIZING', tone: 'info' }
              : progress.state === 'failed'
                ? { label: '✗ FAILED', tone: 'danger' }
                : { label: '· CANCELLED', tone: 'muted' }
  );
</script>

<div class="row" data-state={progress.state}>
  <div class="thumb">
    {#if thumbDataUrl}
      <img src={thumbDataUrl} alt="" />
    {:else}
      <span aria-hidden="true" class="thumb-placeholder">{name[0]?.toUpperCase() ?? '·'}</span>
    {/if}
  </div>

  <div class="meta">
    <p class="filename t-display">{name}</p>
    <p class="t-meta size-line">
      {sizeMb} MB{#if shortHash} · <span class="hash">sha256:{shortHash}</span>{/if}
    </p>

    {#if progress.state === 'uploading'}
      <div
        class="bar"
        role="progressbar"
        aria-valuenow={progress.pct}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label="Upload progress"
      >
        <div class="bar-fill" style:width={`${progress.pct}%`}></div>
      </div>
    {/if}

    <div class="state-line">
      <span class="pill pill-{pill.tone}">{pill.label}</span>
      {#if progress.state === 'uploading'}
        <span class="pct">{Math.round(progress.pct)}%</span>
      {/if}
      {#if progress.state === 'failed' && progress.reason}
        <span class="reason">{progress.reason}</span>
      {/if}
    </div>
  </div>

  <div class="row-actions">
    {#if progress.state === 'ready' && progress.photoId}
      <a class="btn-edit" href={`/upload/${progress.photoId}/verify`} aria-label="Edit metadata">
        ✏ Edit
      </a>
    {/if}
    {#if progress.state === 'failed' && onRetry}
      <button type="button" class="btn-retry" onclick={onRetry}>↻ Retry</button>
    {/if}
    {#if cancellable && onCancel}
      <button
        type="button"
        class="icon-btn"
        onclick={onCancel}
        aria-label="Cancel upload"
        title="Cancel">×</button
      >
    {/if}
  </div>
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: 72px 1fr auto;
    gap: 16px;
    padding: 16px;
    border-bottom: 1px dashed var(--border-subtle);
    align-items: center;
  }
  .row[data-state='failed'] {
    background: color-mix(in oklab, var(--danger, #c33) 8%, transparent);
  }
  .row[data-state='cancelled'] {
    opacity: 0.6;
  }

  .row-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .btn-edit,
  .btn-retry {
    display: inline-flex;
    align-items: center;
    height: 28px;
    padding: 0 10px;
    background: transparent;
    border: 1px solid var(--border-default);
    color: var(--fg-secondary);
    font-family: var(--font-mono);
    font-size: 11px;
    text-decoration: none;
    cursor: pointer;
    letter-spacing: 0.04em;
  }
  .btn-edit:hover,
  .btn-retry:hover {
    color: var(--accent);
    border-color: var(--accent);
  }

  .icon-btn {
    background: transparent;
    border: 1px solid var(--border-default);
    color: var(--fg-muted);
    width: 28px;
    height: 28px;
    line-height: 1;
    cursor: pointer;
    font-size: 16px;
  }

  .icon-btn:hover {
    color: var(--danger);
    border-color: var(--danger);
  }

  .thumb {
    width: 72px;
    height: 72px;
    background: var(--bg-base);
    border: 1px solid var(--border-subtle);
    display: grid;
    place-items: center;
    overflow: hidden;
    flex-shrink: 0;
  }
  .thumb-placeholder {
    color: var(--fg-muted);
    font-family: var(--font-display);
    font-size: 28px;
    font-style: italic;
  }

  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .meta {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 4px;
    min-width: 0;
  }

  .filename {
    font-style: italic;
    font-size: 15px;
    font-family: var(--font-display);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin: 0;
  }

  .size-line {
    margin: 0;
  }
  .hash {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
  }

  .bar {
    height: 4px;
    background: var(--bg-base);
    margin-top: 6px;
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.15s;
  }

  .state-line {
    margin-top: 6px;
    display: flex;
    align-items: center;
    gap: 12px;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.06em;
  }
  .pill {
    padding: 1px 8px;
    border: 1px solid currentColor;
    border-radius: 3px;
    text-transform: uppercase;
  }
  .pill-success {
    color: var(--success, #2a9d4a);
  }
  .pill-accent {
    color: var(--accent);
  }
  .pill-info {
    color: var(--info, #4a8fa1);
  }
  .pill-danger {
    color: var(--danger, #c33);
  }
  .pill-muted {
    color: var(--fg-muted);
  }
  .pct {
    color: var(--accent);
    font-variant-numeric: tabular-nums;
  }
  .reason {
    color: var(--danger, #c33);
    text-transform: none;
    letter-spacing: 0;
  }
</style>
