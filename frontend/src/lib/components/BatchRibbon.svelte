<script lang="ts">
  import Img from '$lib/components/Img.svelte';
  import type { PhotoDetail } from '$lib/api/PhotoDetail';

  interface Props {
    photos: PhotoDetail[];
    selectedId: string;
    onSelect: (id: string) => void;
  }

  let { photos, selectedId, onSelect }: Props = $props();

  function statusPip(p: PhotoDetail): string {
    if (p.status === 'processing') return '⟳';
    if (p.status === 'failed') return '✗';
    if (p.status === 'ready') return '✓';
    return '—';
  }

  let currentIndex = $derived(photos.findIndex((p) => p.id === selectedId));

  function selectByOffset(off: number) {
    const next = Math.min(Math.max(currentIndex + off, 0), photos.length - 1);
    const target = photos[next];
    if (target) onSelect(target.id);
  }

  function onKey(e: KeyboardEvent) {
    const target = e.target as HTMLElement | null;
    const tag = target?.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || target?.isContentEditable) return;
    if (e.key === 'ArrowLeft') {
      e.preventDefault();
      selectByOffset(-1);
    } else if (e.key === 'ArrowRight') {
      e.preventDefault();
      selectByOffset(1);
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<nav class="ribbon" aria-label="Photos in this batch">
  {#each photos as p, i}
    <button
      type="button"
      class="thumb"
      class:current={p.id === selectedId}
      data-status={p.status}
      onclick={() => onSelect(p.id)}
      aria-current={p.id === selectedId ? 'true' : undefined}
      aria-label={`Photo ${i + 1} of ${photos.length}: ${p.original_name} (${p.status})`}
    >
      <Img photoId={p.id} w={144} alt={p.original_name} />
      <span class="pip" aria-hidden="true">{statusPip(p)}</span>
    </button>
  {/each}
</nav>

<div class="ribbon-meta">
  <button
    type="button"
    class="nav-btn"
    onclick={() => selectByOffset(-1)}
    disabled={currentIndex <= 0}>← Prev</button
  >
  <span class="t-meta">{currentIndex + 1} of {photos.length}</span>
  <button
    type="button"
    class="nav-btn"
    onclick={() => selectByOffset(1)}
    disabled={currentIndex >= photos.length - 1}>Next →</button
  >
</div>

<style>
  .ribbon {
    display: flex;
    gap: 8px;
    overflow-x: auto;
    padding: 8px 0;
  }
  .thumb {
    position: relative;
    flex: 0 0 auto;
    width: 64px;
    height: 64px;
    overflow: hidden;
    cursor: pointer;
    background: transparent;
    border: 2px solid transparent;
    padding: 0;
  }
  .thumb.current {
    border-color: var(--accent);
  }
  .thumb :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .thumb[data-status='failed'] {
    opacity: 0.6;
  }
  .pip {
    position: absolute;
    bottom: 2px;
    right: 2px;
    width: 16px;
    height: 16px;
    line-height: 16px;
    text-align: center;
    font-family: var(--font-mono);
    font-size: 10px;
    background: rgba(12, 10, 8, 0.85);
    color: var(--accent);
  }
  .ribbon-meta {
    display: flex;
    gap: 16px;
    align-items: center;
    padding: 8px 0;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
  }
  .nav-btn {
    background: transparent;
    border: 1px solid var(--border-default);
    color: var(--fg-primary);
    padding: 4px 12px;
    cursor: pointer;
    font-family: var(--font-mono);
    font-size: 11px;
  }
  .nav-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
</style>
