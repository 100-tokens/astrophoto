<script lang="ts">
  import { goto } from '$app/navigation';
  import { tick } from 'svelte';
  import { fetchSearch } from '$lib/api/discoveryClient';
  import type { SearchResults } from '$lib/api/SearchResults';
  import SuggestionsList from './SuggestionsList.svelte';

  let inputEl: HTMLInputElement | null = null;
  let query = $state('');
  let focused = $state(false);
  let results = $state<SearchResults | null>(null);
  let focusedIdx = $state(-1);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  // Mobile-only: when true the inline search-wrap takes over the viewport
  // as a fullscreen overlay. Desktop ignores this state.
  let mobileOpen = $state(false);

  async function openMobile() {
    mobileOpen = true;
    await tick();
    inputEl?.focus();
  }
  function closeMobile() {
    mobileOpen = false;
    query = '';
    results = null;
    focused = false;
  }

  const EMPTY_RESULTS: SearchResults = { q: '', targets: [], users: [], photos: [] };

  let showSuggestions = $derived(focused && query.trim().length > 0 && results !== null);

  // Flattened item count for keyboard nav bounds
  let itemCount = $derived(
    results ? results.targets.length + results.users.length + results.photos.length : 0
  );

  function handleInput() {
    focusedIdx = -1;
    if (debounceTimer !== null) clearTimeout(debounceTimer);
    const q = query.trim();
    if (q.length === 0) {
      results = null;
      return;
    }
    debounceTimer = setTimeout(() => {
      void fetchResults(q);
    }, 300);
  }

  async function fetchResults(q: string) {
    try {
      results = await fetchSearch(fetch, q);
    } catch {
      results = EMPTY_RESULTS;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!showSuggestions) {
      if (e.key === 'Escape') {
        inputEl?.blur();
        query = '';
        results = null;
        if (mobileOpen) mobileOpen = false;
      }
      return;
    }

    if (e.key === 'ArrowDown') {
      e.preventDefault();
      focusedIdx = Math.min(focusedIdx + 1, itemCount - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      focusedIdx = Math.max(focusedIdx - 1, -1);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      inputEl?.blur();
      focused = false;
      results = null;
      query = '';
      if (mobileOpen) mobileOpen = false;
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (focusedIdx >= 0 && results) {
        navigateToFocused();
      } else {
        const q = query.trim();
        if (q) {
          closeSuggestions();
          void goto(`/search?q=${encodeURIComponent(q)}`);
        }
      }
    }
  }

  function navigateToFocused() {
    if (!results) return;
    const tLen = results.targets.length;
    const uLen = results.users.length;
    const idx = focusedIdx;
    closeSuggestions();
    if (idx < tLen) {
      const t = results.targets[idx];
      if (t) void goto(`/t/${t.slug}`);
    } else if (idx < tLen + uLen) {
      const u = results.users[idx - tLen];
      if (u) void goto(`/u/${u.handle}`);
    } else {
      const p = results.photos[idx - tLen - uLen];
      if (p) void goto(`/u/${p.author_handle}/p/${p.short_id}`);
    }
  }

  function closeSuggestions() {
    focused = false;
    results = null;
    query = '';
    inputEl?.blur();
    if (mobileOpen) mobileOpen = false;
  }

  // Global ⌘K / Ctrl-K handler.
  $effect(() => {
    function onGlobalKeydown(e: KeyboardEvent) {
      // Skip when any input/textarea/contenteditable has focus.
      const target = e.target as HTMLElement | null;
      if (
        target &&
        (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable)
      ) {
        return;
      }
      if (e.key === 'k' && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        inputEl?.focus();
      }
    }
    window.addEventListener('keydown', onGlobalKeydown);
    return () => window.removeEventListener('keydown', onGlobalKeydown);
  });
</script>

<!-- Mobile-only icon trigger. Shown under 640 px; tapping opens the
     fullscreen overlay below. -->
<button type="button" class="search-trigger-mobile" aria-label="Open search" onclick={openMobile}>
  <svg
    width="18"
    height="18"
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    stroke-width="1.4"
    aria-hidden="true"
  >
    <circle cx="7" cy="7" r="5" />
    <line x1="11" y1="11" x2="14" y2="14" />
  </svg>
</button>

<div class="search-wrap" class:search-wrap-mobile-open={mobileOpen}>
  {#if mobileOpen}
    <button type="button" class="mobile-scrim" aria-label="Close search" onclick={closeMobile}
    ></button>
  {/if}
  <div class="search-box" class:search-box-focused={focused}>
    <svg
      width="12"
      height="12"
      viewBox="0 0 16 16"
      fill="none"
      stroke={focused ? 'var(--accent)' : 'currentColor'}
      stroke-width="1.2"
      aria-hidden="true"
    >
      <circle cx="7" cy="7" r="5" />
      <line x1="11" y1="11" x2="14" y2="14" />
    </svg>
    <input
      bind:this={inputEl}
      bind:value={query}
      type="search"
      name="q"
      id="global-search"
      class="search-input"
      placeholder="search the archive…"
      autocomplete="off"
      spellcheck={false}
      aria-label="Search"
      role="combobox"
      aria-autocomplete="list"
      aria-expanded={showSuggestions}
      aria-controls="global-search-listbox"
      aria-activedescendant={showSuggestions && focusedIdx >= 0
        ? `global-search-opt-${focusedIdx}`
        : undefined}
      onfocus={() => {
        focused = true;
      }}
      onblur={() => {
        // Delay so clicks on suggestions register first.
        setTimeout(() => {
          focused = false;
        }, 150);
      }}
      oninput={handleInput}
      onkeydown={handleKeydown}
    />
    {#if mobileOpen}
      <button type="button" class="mobile-close" aria-label="Close" onclick={closeMobile}>×</button>
    {:else if !focused || query.length === 0}
      <span class="kbd-hint">⌘K</span>
    {/if}
  </div>

  {#if showSuggestions && results}
    <SuggestionsList
      {results}
      focusedIndex={focusedIdx}
      onFocusChange={(idx) => {
        focusedIdx = idx;
      }}
      onClose={closeSuggestions}
    />
  {/if}
</div>

<style>
  .search-wrap {
    position: relative;
    width: 220px;
  }

  /* Mobile-only icon trigger — hidden on desktop, visible under 640 px
     in place of the inline search box. */
  .search-trigger-mobile {
    display: none;
    background: transparent;
    border: 1px solid var(--border-default);
    color: var(--fg-secondary);
    width: 32px;
    height: 32px;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    padding: 0;
  }
  .search-trigger-mobile:hover {
    color: var(--accent);
    border-color: var(--accent);
  }

  @media (max-width: 640px) {
    /* Default: inline box hidden, icon trigger shown. */
    .search-wrap {
      display: none;
    }
    .search-trigger-mobile {
      display: inline-flex;
    }
    /* Tap-to-open: search-wrap becomes a fullscreen overlay anchored
       to the top so the on-screen keyboard appears below the input. */
    .search-wrap.search-wrap-mobile-open {
      display: block;
      position: fixed;
      inset: 0;
      width: 100%;
      z-index: 1000;
      padding: 16px;
      box-sizing: border-box;
    }
    .search-wrap.search-wrap-mobile-open .search-box {
      height: 44px;
      font-size: 14px;
    }
    .search-wrap.search-wrap-mobile-open .search-input {
      font-size: 16px;
    }
  }

  .mobile-scrim {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: 0;
    padding: 0;
    margin: 0;
    cursor: default;
    z-index: -1;
  }
  .mobile-close {
    background: transparent;
    border: 0;
    color: var(--fg-muted);
    font-size: 22px;
    line-height: 1;
    width: 28px;
    height: 28px;
    cursor: pointer;
    flex-shrink: 0;
  }

  .search-box {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    height: 32px;
    border: 1px solid var(--border-default);
    color: var(--fg-muted);
    font-family: var(--font-mono);
    font-size: 12px;
    width: 100%;
    box-sizing: border-box;
    transition: border-color 0.1s;
  }

  .search-box-focused {
    border-color: var(--accent);
    color: var(--fg-primary);
  }

  .search-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-primary);
    min-width: 0;
    /* Remove default search input styling */
    -webkit-appearance: none;
    appearance: none;
  }

  .search-input::placeholder {
    color: var(--fg-muted);
  }

  /* Hide browser's native clear button */
  .search-input::-webkit-search-cancel-button {
    -webkit-appearance: none;
    display: none;
  }

  .kbd-hint {
    margin-left: auto;
    font-size: 10px;
    letter-spacing: 0.1em;
    color: var(--fg-muted);
    flex-shrink: 0;
  }
</style>
