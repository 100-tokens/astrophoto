<script lang="ts">
  import type { SearchResults } from '$lib/api/SearchResults';
  import { goto } from '$app/navigation';

  let {
    results,
    focusedIndex = -1,
    query = '',
    overlay = false,
    onFocusChange,
    onClose,
    onSeeAll
  }: {
    results: SearchResults;
    focusedIndex?: number;
    query?: string;
    overlay?: boolean;
    onFocusChange?: (idx: number) => void;
    onClose?: () => void;
    onSeeAll?: () => void;
  } = $props();

  let totalTargets = $derived(results.targets.length);
  let totalUsers = $derived(results.users.length);
  let root: HTMLDivElement | undefined = $state();

  // Pin under the real header box (grace banner sits above `.app-header`)
  // or, in the mobile overlay, under the search field itself.
  $effect(() => {
    const node = root;
    if (!node) return;
    const isOverlay = overlay;
    function apply() {
      const anchor = isOverlay
        ? node.parentElement?.querySelector('.search-box')
        : document.querySelector('header.app-header');
      if (!(anchor instanceof HTMLElement)) return;
      node.style.top = `${anchor.getBoundingClientRect().bottom}px`;
    }
    apply();
    const ro = new ResizeObserver(apply);
    const header = document.querySelector('header.app-header');
    const banner = document.querySelector('.grace-banner');
    const box = node.parentElement?.querySelector('.search-box');
    if (header) ro.observe(header);
    if (banner) ro.observe(banner);
    if (box) ro.observe(box);
    window.addEventListener('resize', apply);
    window.visualViewport?.addEventListener('resize', apply);
    return () => {
      ro.disconnect();
      window.removeEventListener('resize', apply);
      window.visualViewport?.removeEventListener('resize', apply);
    };
  });

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

<div
  bind:this={root}
  class="suggestions"
  id="global-search-listbox"
  role="listbox"
  aria-label="Search suggestions"
>
  {#if results.targets.length > 0}
    <div class="bucket">
      <div class="bucket-label">● TARGETS · {results.targets.length}</div>
      {#each results.targets as t, i}
        <button
          type="button"
          class="drop-row"
          class:drop-row-focused={focusedIndex === i}
          id={`global-search-opt-${i}`}
          role="option"
          aria-selected={focusedIndex === i}
          onmouseenter={() => onFocusChange?.(i)}
          onclick={() => navigateTarget(t.slug)}
        >
          <span class="target-slug" class:target-slug-focused={focusedIndex === i}>
            {t.slug.toUpperCase()}
          </span>
          <span class="item-name">{t.canonical_name}</span>
          <span class="item-meta"
            >{Number(t.photo_count)} {Number(t.photo_count) === 1 ? 'PHOTO' : 'PHOTOS'}</span
          >
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
          id={`global-search-opt-${idx}`}
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
          id={`global-search-opt-${idx}`}
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

  <div class="footer" role="presentation">
    <span class="footer-hint">↑↓ NAVIGATE · ↩ OPEN · ESC CLOSE</span>
    <a
      class="footer-all"
      href="/search?q={encodeURIComponent(query)}"
      onclick={(e) => {
        e.preventDefault();
        onSeeAll?.();
      }}>SEE ALL RESULTS →</a
    >
  </div>
</div>

<style>
  /* Full-width destination panel under the header — names must not truncate
     in a 220px popover over the hero. */
  .suggestions {
    position: fixed;
    left: 0;
    right: 0;
    top: 64px;
    width: 100%;
    max-height: min(70vh, 640px);
    overflow-y: auto;
    background: var(--bg-elevated);
    border: 1px solid var(--border-default);
    border-left: 0;
    border-right: 0;
    box-shadow: var(--shadow-lg, 0 8px 24px rgba(0, 0, 0, 0.4));
    z-index: 200;
  }

  .bucket {
    border-bottom: 1px dashed var(--border-default);
    padding: 8px 64px;
  }

  .bucket-label {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    color: var(--accent);
    padding: 4px 0;
  }

  .drop-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
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
    min-width: 0;
    white-space: normal;
    overflow: visible;
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
    padding: 12px 64px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
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
    text-decoration: none;
  }

  @media (max-width: 640px) {
    .bucket,
    .footer {
      padding-left: 16px;
      padding-right: 16px;
    }
  }
</style>
