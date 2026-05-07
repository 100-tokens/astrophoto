<script lang="ts">
  import type { SlotProgress } from '$lib/upload/pump';

  interface Props {
    name: string;
    size: number;
    thumbDataUrl?: string;
    progress: SlotProgress;
    onRetry?: () => void;
    onCancel?: () => void;
  }

  let { name, size, thumbDataUrl, progress, onRetry, onCancel }: Props = $props();

  const sizeMb = $derived((size / 1024 / 1024).toFixed(1));
  const cancellable = $derived(['hashing', 'queued', 'uploading'].includes(progress.state));
</script>

<div class="row" data-state={progress.state}>
  <div class="thumb">
    {#if thumbDataUrl}
      <img src={thumbDataUrl} alt="" />
    {:else}
      <span aria-hidden="true">🖼</span>
    {/if}
  </div>

  <div class="meta">
    <p class="filename t-display">{name}</p>
    <p class="t-meta size-line">{sizeMb} MB</p>

    {#if progress.state === 'queued'}
      <p class="t-meta status-muted">queued</p>
    {:else if progress.state === 'hashing'}
      <p class="t-meta status-muted">hashing…</p>
    {:else if progress.state === 'uploading'}
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
    {:else if progress.state === 'finalizing'}
      <p class="t-meta status-muted">finalizing…</p>
    {:else if progress.state === 'ready'}
      <a class="t-meta action-link" href={`/upload/${progress.photoId}/verify`}
        >✓ Saved as draft · Continue to verify →</a
      >
    {:else if progress.state === 'failed'}
      <span class="chip chip-failed">✗ {progress.reason ?? 'Failed'}</span>
      {#if onRetry}
        <button type="button" class="chip-action" onclick={onRetry} aria-label="Retry upload">↻ Retry</button>
      {/if}
    {:else if progress.state === 'cancelled'}
      <p class="t-meta status-muted">cancelled</p>
    {/if}
  </div>

  <div class="row-actions">
    {#if cancellable && onCancel}
      <button type="button" class="icon-btn" onclick={onCancel} aria-label="Cancel upload" title="Cancel">×</button>
    {/if}
  </div>
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: 64px 1fr auto;
    gap: 12px;
    padding: 12px 0;
    border-bottom: 1px solid var(--border-subtle);
  }

  .row-actions {
    display: flex;
    align-items: center;
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

  .chip-action {
    display: inline-block;
    padding: 1px 6px;
    border: 1px solid currentColor;
    background: transparent;
    color: var(--accent);
    font-size: 12px;
    margin-left: 8px;
    cursor: pointer;
  }

  .thumb {
    width: 64px;
    height: 64px;
    background: var(--bg-base);
    display: grid;
    place-items: center;
    overflow: hidden;
    flex-shrink: 0;
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

  .status-muted {
    color: var(--fg-muted);
    margin: 0;
  }

  .bar {
    height: 3px;
    background: var(--bg-base);
    margin-top: 4px;
    border-radius: 2px;
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.15s;
    border-radius: 2px;
  }

  .action-link {
    color: var(--accent);
    text-decoration: none;
  }

  .action-link:hover {
    text-decoration: underline;
  }

  .chip {
    display: inline-block;
    padding: 1px 6px;
    border: 1px solid currentColor;
    border-radius: 3px;
    font-size: 12px;
    line-height: 1.5;
  }

  .chip-failed {
    color: var(--danger);
  }
</style>
