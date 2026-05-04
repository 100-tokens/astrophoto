<script lang="ts">
  import { goto } from '$app/navigation';
  import type { SearchResults } from '$lib/api/SearchResults';
  import SuggestionsList from './SuggestionsList.svelte';

  let inputEl: HTMLInputElement | null = null;
  let query = $state('');
  let focused = $state(false);
  let results = $state<SearchResults | null>(null);
  let focusedIdx = $state(-1);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

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
      const r = await fetch(`/api/search?q=${encodeURIComponent(q)}`);
      if (!r.ok) {
        results = EMPTY_RESULTS;
        return;
      }
      results = (await r.json()) as SearchResults;
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

<div class="search-wrap">
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
      class="search-input"
      placeholder="search the archive…"
      autocomplete="off"
      spellcheck={false}
      aria-label="Search"
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
    {#if !focused || query.length === 0}
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
