<script lang="ts">
  import type { SearchResults } from '$lib/api/SearchResults';
  import { goto } from '$app/navigation';

  let {
    results,
    focusedIndex = -1,
    onFocusChange,
    onClose
  }: {
    results: SearchResults;
    focusedIndex?: number;
    onFocusChange?: (idx: number) => void;
    onClose?: () => void;
  } = $props();

  let totalTargets = $derived(results.targets.length);
  let totalUsers = $derived(results.users.length);

  function navigateTarget(slug: string) {
    onClose?.();
    void goto(`/t/${slug}`);
  }

  function navigateUser(handle: string) {
    onClose?.();
    void goto(`/u/${handle}`);
  }

  function navigatePhoto(authorHandle: string, shortId: string) {
    onClose?.();
    void goto(`/u/${authorHandle}/p/${shortId}`);
  }
</script>

<div class="suggestions" role="listbox" aria-label="Search suggestions">
  {#if results.targets.length > 0}
    <div class="bucket">
      <div class="bucket-label">● TARGETS · {results.targets.length}</div>
      {#each results.targets as t, i}
        <button
          type="button"
          class="drop-row"
          class:drop-row-focused={focusedIndex === i}
          role="option"
          aria-selected={focusedIndex === i}
          onmouseenter={() => onFocusChange?.(i)}
          onclick={() => navigateTarget(t.slug)}
        >
          <span class="target-slug" class:target-slug-focused={focusedIndex === i}>
            {t.slug.toUpperCase()}
          </span>
          <span class="item-name">{t.canonical_name}</span>
          <span class="item-meta">{Number(t.photo_count)} PHOTOS</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if results.users.length > 0}
    <div class="bucket">
      <div class="bucket-label">● PHOTOGRAPHERS · {results.users.length}</div>
      {#each results.users as u, i}
        {@const idx = totalTargets + i}
        <button
          type="button"
          class="drop-row"
          class:drop-row-focused={focusedIndex === idx}
          role="option"
          aria-selected={focusedIndex === idx}
          onmouseenter={() => onFocusChange?.(idx)}
          onclick={() => navigateUser(u.handle)}
        >
          <div class="avatar">{u.display_name[0]?.toUpperCase() ?? '?'}</div>
          <span class="item-name">{u.display_name}</span>
          <span class="item-meta">@{u.handle.toUpperCase()}</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if results.photos.length > 0}
    <div class="bucket">
      <div class="bucket-label">● PHOTOS · {results.photos.length}</div>
      {#each results.photos.slice(0, 4) as p, i}
        {@const idx = totalTargets + totalUsers + i}
        <button
          type="button"
          class="drop-row"
          class:drop-row-focused={focusedIndex === idx}
          role="option"
          aria-selected={focusedIndex === idx}
          onmouseenter={() => onFocusChange?.(idx)}
          onclick={() => navigatePhoto(p.author_handle, p.short_id)}
        >
          <span class="item-name">{p.target ?? 'Untitled'}</span>
          <span class="item-meta">@{p.author_handle.toUpperCase()}</span>
        </button>
      {/each}
    </div>
  {/if}

  <div class="footer">
    <span class="footer-hint">↑↓ NAVIGATE · ↩ OPEN · ESC CLOSE</span>
    <span class="footer-all">SEE ALL →</span>
  </div>
</div>

<style>
  .suggestions {
    position: absolute;
    left: 0;
    right: 0;
    top: calc(100% + 4px);
    background: var(--bg-elevated);
    border: 1px solid var(--border-default);
    box-shadow: var(--shadow-lg, 0 8px 24px rgba(0, 0, 0, 0.4));
    z-index: 100;
  }

  .bucket {
    border-bottom: 1px dashed var(--border-default);
    padding: 8px 0;
  }

  .bucket-label {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    color: var(--accent);
    padding: 4px 12px;
  }

  .drop-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    width: 100%;
    background: transparent;
    border: none;
    border-left: 2px solid transparent;
    color: var(--fg-primary);
    cursor: pointer;
    text-align: left;
  }

  .drop-row-focused {
    background: var(--bg-accent-tint, rgba(180, 150, 60, 0.08));
    border-left-color: var(--accent);
  }

  .drop-row:hover {
    background: var(--bg-accent-tint, rgba(180, 150, 60, 0.08));
  }

  .target-slug {
    font-family: var(--font-mono);
    font-size: 13px;
    min-width: 56px;
    flex-shrink: 0;
    color: var(--fg-secondary);
  }

  .target-slug-focused {
    color: var(--accent);
  }

  .item-name {
    font-family: var(--font-display);
    font-style: italic;
    font-size: 14px;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .item-meta {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.06em;
    color: var(--fg-muted);
    flex-shrink: 0;
  }

  .avatar {
    width: 24px;
    height: 24px;
    background: var(--accent-dim, rgba(180, 150, 60, 0.3));
    color: var(--accent-ink, #fff);
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-display);
    font-size: 12px;
    flex-shrink: 0;
  }

  .footer {
    padding: 8px 12px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .footer-hint {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.06em;
    color: var(--fg-muted);
  }

  .footer-all {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.06em;
    color: var(--accent);
    cursor: pointer;
  }
</style>
