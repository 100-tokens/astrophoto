<script lang="ts">
  import FieldShell from './FieldShell.svelte';

  // TagChipInput — lighter chip input visually distinct from the FilterChip
  // system. Each tag becomes a small mono chip prefixed with `#`. Adds on
  // Enter or comma, lowercases + hyphenates whitespace, max 8.
  //
  // Output: serializes to a hidden input as a JSON-stringified string[],
  // matching the `tags` field consumed by collectPatch in +page.server.ts.

  interface Props {
    name?: string;
    value?: string[];
    suggestions?: string[];
  }

  const MAX = 8;
  const DEFAULT_SUGGESTIONS = ['mosaic', 'autumn-2025', 'first-light', 'reprocess'];

  let {
    name = 'tags',
    value = $bindable<string[]>([]),
    suggestions = DEFAULT_SUGGESTIONS
  }: Props = $props();

  let buf = $state('');
  let inputEl = $state<HTMLInputElement | undefined>(undefined);

  function normalize(s: string): string {
    return s
      .trim()
      .toLowerCase()
      .replace(/\s+/g, '-')
      .replace(/[^a-z0-9_-]+/g, '');
  }

  function add(raw: string) {
    const v = normalize(raw);
    if (!v) return;
    if (value.includes(v)) {
      buf = '';
      return;
    }
    if (value.length >= MAX) return;
    value = [...value, v];
    buf = '';
  }

  function remove(t: string) {
    value = value.filter((x) => x !== t);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ',') {
      e.preventDefault();
      add(buf);
    } else if (e.key === 'Backspace' && !buf && value.length) {
      const last = value[value.length - 1];
      if (last !== undefined) remove(last);
    }
  }

  let unusedSuggestions = $derived(suggestions.filter((s) => !value.includes(s)));
  let atMax = $derived(value.length >= MAX);
</script>

<FieldShell label={`TAGS · MAX ${MAX}`} hint="Free-form keywords. Lowercase, hyphenated.">
  <div class="fchip-input tag-input" onclick={() => inputEl?.focus()} role="presentation">
    {#each value as t (t)}
      <span class="tag-chip">
        <span class="tag-hash" aria-hidden="true">#</span>{t}
        <button
          type="button"
          class="tag-x"
          aria-label={`remove ${t}`}
          onclick={(e) => {
            e.stopPropagation();
            remove(t);
          }}
        >
          <svg width="8" height="8" viewBox="0 0 9 9" aria-hidden="true">
            <path
              d="M2 2 L7 7 M7 2 L2 7"
              stroke="currentColor"
              stroke-width="1.4"
              stroke-linecap="round"
              fill="none"
            />
          </svg>
        </button>
      </span>
    {/each}
    <input
      bind:this={inputEl}
      bind:value={buf}
      onkeydown={onKey}
      onblur={() => add(buf)}
      placeholder={atMax ? 'limit reached — backspace to remove' : 'Type a tag, press ↵'}
      autocomplete="off"
      spellcheck="false"
      disabled={atMax}
      aria-label="Add tag"
    />
  </div>
  <input type="hidden" {name} value={JSON.stringify(value)} />
  <div class="tag-meta">
    <span class="t-meta tag-suggest">
      Suggestions:
      {#each unusedSuggestions as s (s)}
        <button type="button" class="tag-suggest-btn" disabled={atMax} onclick={() => add(s)}
          >#{s}</button
        >
      {/each}
    </span>
    <span class="t-meta tag-counter" class:over={atMax}>
      {value.length} / {MAX}
    </span>
  </div>
</FieldShell>

<style>
  .tag-input {
    min-height: 44px;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
    padding: 6px 8px;
    cursor: text;
  }
  .tag-input input {
    flex: 1;
    min-width: 140px;
    border: 0;
    background: transparent;
    color: var(--fg-primary);
    font-family: var(--font-mono);
    font-size: 12px;
    outline: none;
    padding: 4px;
  }
  .tag-input input::placeholder {
    color: var(--fg-faint);
  }
  .tag-chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 4px 0 8px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: var(--r-sm);
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-secondary);
    letter-spacing: 0.02em;
  }
  .tag-hash {
    color: var(--fg-faint);
  }
  .tag-x {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    height: 14px;
    background: transparent;
    border: 0;
    color: var(--fg-muted);
    cursor: pointer;
    padding: 0;
    line-height: 1;
  }
  .tag-x:hover {
    color: var(--accent);
  }
  .tag-meta {
    margin-top: 6px;
    display: flex;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 8px;
  }
  .tag-suggest {
    color: var(--fg-faint);
  }
  .tag-suggest-btn {
    background: transparent;
    border: 0;
    padding: 0;
    margin-right: 8px;
    cursor: pointer;
    color: var(--fg-muted);
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.02em;
  }
  .tag-suggest-btn:hover:not(:disabled) {
    color: var(--accent);
  }
  .tag-suggest-btn:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }
  .tag-counter {
    color: var(--fg-faint);
  }
  .tag-counter.over {
    color: var(--warning);
  }
</style>
