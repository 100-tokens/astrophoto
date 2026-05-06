<script lang="ts">
  type Target = { slug: string; canonical_name: string; kind: string };

  interface Props {
    placeholder?: string;
    excludeSlugs?: string[];
    api?: string;
    id?: string;
    onPick: (t: Target) => void;
  }
  let {
    placeholder = 'tape pour ajouter…',
    excludeSlugs = [],
    api = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '',
    id = undefined,
    onPick,
  }: Props = $props();

  let query = $state('');
  let suggestions = $state<Target[]>([]);
  let highlighted = $state(-1);

  // Stale-response guard — increment on each new request.
  let reqId = 0;

  // Stable ID for the listbox — required by the combobox ARIA pattern.
  const listboxId = $derived(id ? `${id}-listbox` : 'target-autocomplete-listbox');

  $effect(() => {
    if (!query.trim()) {
      suggestions = [];
      highlighted = -1;
      return;
    }
    const myId = ++reqId;
    const t = setTimeout(async () => {
      try {
        const r = await fetch(`${api}/api/targets/autocomplete?q=${encodeURIComponent(query)}`);
        if (r.ok && myId === reqId) {
          const all = (await r.json()).targets as Target[];
          suggestions = all.filter((s) => !excludeSlugs.includes(s.slug));
          highlighted = -1;
        }
      } catch {
        if (myId === reqId) suggestions = [];
      }
    }, 200);
    return () => clearTimeout(t);
  });

  function pick(s: Target) {
    onPick(s);
    query = '';
    suggestions = [];
    highlighted = -1;
  }

  function onKeydown(e: KeyboardEvent) {
    if (!suggestions.length) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      highlighted = Math.min(highlighted + 1, suggestions.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      highlighted = Math.max(-1, highlighted - 1);
    } else if (e.key === 'Enter' && highlighted >= 0) {
      e.preventDefault();
      const s = suggestions[highlighted];
      if (s) pick(s);
    } else if (e.key === 'Escape') {
      suggestions = [];
      highlighted = -1;
    }
  }

  function onBlur() {
    // Small delay so onmousedown={e.preventDefault()} on <li> can fire first.
    setTimeout(() => {
      suggestions = [];
      highlighted = -1;
    }, 120);
  }
</script>

<div class="ac">
  <input
    {id}
    type="text"
    role="combobox"
    bind:value={query}
    class="input input-mono"
    {placeholder}
    onkeydown={onKeydown}
    onblur={onBlur}
    autocomplete="off"
    spellcheck="false"
    aria-autocomplete="list"
    aria-expanded={suggestions.length > 0}
    aria-controls={listboxId}
  />
  {#if suggestions.length}
    <ul id={listboxId} class="ac-list card" role="listbox">
      {#each suggestions as s, i (s.slug)}
        <!-- onmousedown prevents blur from firing before click, keeping focus intact.
             Keyboard nav (↑↓ Enter Esc) on the <input> above handles all keyboard cases. -->
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

<style>
  .ac {
    position: relative;
  }
  .ac-list {
    position: absolute;
    top: 100%;
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
  }
  .ac-list li:hover,
  .ac-highlighted {
    background: var(--bg-elevated);
  }
</style>
