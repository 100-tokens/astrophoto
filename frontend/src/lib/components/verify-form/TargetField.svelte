<script lang="ts">
  import FieldShell from './FieldShell.svelte';

  // TargetField — the hero input of the verify form. Larger than every other
  // field (48px tall, display-serif italic 20px), with a trailing CLEAR
  // affordance when a value is present. Wraps the existing
  // /api/targets/autocomplete endpoint inline (we re-implement the small
  // combobox here so we can style it precisely; the underlying contract is
  // unchanged from TargetAutocompleteInput).

  type Target = { slug: string; canonical_name: string; kind: string };

  interface Props {
    name?: string;
    value?: string;
    api?: string;
    /** Optional matched-catalog label shown in the trailing meta tag
     *  (e.g. "NORTH AMERICA NEB."). When empty, only the CLEAR button shows. */
    matchedLabel?: string | null;
  }

  let {
    name = 'target',
    value = $bindable(''),
    api = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '',
    matchedLabel = null
  }: Props = $props();

  let query = $state(value);
  let suggestions = $state<Target[]>([]);
  let highlighted = $state(-1);
  let focused = $state(false);
  let reqId = 0;

  // Sync the typed query when the parent forces a new value (e.g. detach setup).
  $effect(() => {
    const v = value;
    if (v !== query && !focused) {
      query = v;
    }
  });

  $effect(() => {
    const q = query.trim();
    if (!q) {
      suggestions = [];
      highlighted = -1;
      return;
    }
    const myId = ++reqId;
    const t = setTimeout(async () => {
      try {
        const r = await fetch(`${api}/api/targets/autocomplete?q=${encodeURIComponent(q)}`);
        if (r.ok && myId === reqId) {
          suggestions = (await r.json()).targets as Target[];
          highlighted = -1;
        }
      } catch {
        if (myId === reqId) suggestions = [];
      }
    }, 200);
    return () => clearTimeout(t);
  });

  function pick(t: Target) {
    value = t.canonical_name;
    query = t.canonical_name;
    suggestions = [];
    highlighted = -1;
  }

  function commitFreetext() {
    const trimmed = query.trim();
    if (trimmed) value = trimmed;
  }

  function clear() {
    value = '';
    query = '';
    suggestions = [];
    highlighted = -1;
    const el = document.getElementById(name) as HTMLInputElement | null;
    el?.focus();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (suggestions.length) highlighted = Math.min(highlighted + 1, suggestions.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (suggestions.length) highlighted = Math.max(-1, highlighted - 1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (highlighted >= 0) {
        const s = suggestions[highlighted];
        if (s) pick(s);
      } else {
        commitFreetext();
      }
    } else if (e.key === 'Escape') {
      suggestions = [];
      highlighted = -1;
    }
  }

  function onBlur() {
    // Delay so click on <li> lands first.
    setTimeout(() => {
      focused = false;
      if (query.trim() && !suggestions.find((s) => s.canonical_name === value)) {
        commitFreetext();
      }
      suggestions = [];
      highlighted = -1;
    }, 120);
  }
</script>

<FieldShell label="TARGET" hint="Catalog autocomplete — type to search Messier, NGC, IC, Sh2 …">
  <div class="tf-target">
    <input
      id={name}
      type="text"
      role="combobox"
      class="input tf-target-input"
      bind:value={query}
      placeholder="Search catalog targets…"
      onkeydown={onKeydown}
      onfocus={() => {
        focused = true;
      }}
      onblur={onBlur}
      autocomplete="off"
      spellcheck="false"
      aria-autocomplete="list"
      aria-expanded={suggestions.length > 0}
      aria-controls={`${name}-listbox`}
    />
    {#if query}
      <div class="tf-target-trail">
        {#if matchedLabel}
          <span class="tf-target-meta t-meta">{matchedLabel}</span>
        {/if}
        <button
          type="button"
          class="btn btn-ghost btn-sm tf-target-clear"
          onclick={clear}
          aria-label="Clear target"
        >
          CLEAR
        </button>
      </div>
    {/if}
    {#if suggestions.length}
      <ul class="ac-list card" id={`${name}-listbox`} role="listbox">
        {#each suggestions as s, i (s.slug)}
          <li
            role="option"
            aria-selected={i === highlighted}
            class:ac-highlighted={i === highlighted}
            onmousedown={(e) => e.preventDefault()}
            onclick={() => pick(s)}
            onkeydown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') pick(s);
            }}
          >
            <span class="t-mono">{s.slug}</span> · {s.canonical_name}
          </li>
        {/each}
      </ul>
    {/if}
  </div>
  <input type="hidden" {name} {value} />
</FieldShell>

<style>
  .tf-target {
    position: relative;
  }
  .tf-target-input {
    height: 48px;
    padding-left: 14px;
    padding-right: 96px;
    font-family: var(--font-display);
    font-size: 20px;
    font-style: italic;
    background: var(--bg-base);
  }
  .tf-target-input:not(:placeholder-shown) {
    font-style: italic;
  }
  .tf-target-trail {
    position: absolute;
    right: 8px;
    top: 0;
    bottom: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    pointer-events: none;
  }
  .tf-target-trail .btn,
  .tf-target-trail .tf-target-meta {
    pointer-events: auto;
  }
  .tf-target-meta {
    color: var(--fg-faint);
  }
  .tf-target-clear {
    height: 24px;
    padding: 0 8px;
    font-size: 10px;
    letter-spacing: 0.1em;
  }
  .ac-list {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    padding: 4px 0;
    max-height: 240px;
    overflow-y: auto;
    z-index: 10;
  }
  .ac-list li {
    padding: 6px 12px;
    cursor: pointer;
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .ac-list li:hover,
  .ac-highlighted {
    background: var(--bg-elevated);
  }
</style>
