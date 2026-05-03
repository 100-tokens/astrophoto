<script lang="ts">
  interface Props {
    name?: string;
    value?: string;
    api?: string;
  }

  let {
    name = 'target',
    value = $bindable(''),
    api = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '',
  }: Props = $props();

  type Target = { slug: string; canonical_name: string; kind: string };
  let suggestions = $state<Target[]>([]);
  let highlighted = $state(-1);
  let lastSelected = $state('');

  // Stale-response guard — same reqId pattern as HandlePicker.
  let reqId = 0;

  $effect(() => {
    // Suppress re-fetch when the user just selected a suggestion.
    if (!value || value === lastSelected) {
      suggestions = [];
      highlighted = -1;
      return;
    }
    const myId = ++reqId;
    const t = setTimeout(async () => {
      try {
        const r = await fetch(
          `${api}/api/targets/autocomplete?q=${encodeURIComponent(value)}`,
        );
        if (r.ok && myId === reqId) {
          suggestions = (await r.json()).targets;
          highlighted = -1;
        }
      } catch {
        if (myId === reqId) suggestions = [];
      }
    }, 200);
    return () => clearTimeout(t);
  });

  function select(s: Target) {
    lastSelected = s.canonical_name;
    value = s.canonical_name;
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
      if (s) select(s);
    } else if (e.key === 'Escape') {
      suggestions = [];
      highlighted = -1;
    }
  }

  function onBlur() {
    // Small delay so onmousedown={e.preventDefault()} on <li> can fire first.
    setTimeout(() => { suggestions = []; highlighted = -1; }, 120);
  }
</script>

<label class="t-label" for={name}>TARGET</label>
<div class="ac">
  <input
    id={name}
    {name}
    bind:value
    class="input input-mono"
    placeholder="M31, NGC 7000…"
    onkeydown={onKeydown}
    onblur={onBlur}
    autocomplete="off"
    spellcheck="false"
    aria-autocomplete="list"
    aria-expanded={suggestions.length > 0}
  />
  {#if suggestions.length}
    <ul class="ac-list card" role="listbox">
      {#each suggestions as s, i}
        <!-- onmousedown prevents blur from firing before click, keeping focus intact.
             Keyboard nav (↑↓ Enter Esc) on the <input> above handles all keyboard cases. -->
        <li
          role="option"
          aria-selected={i === highlighted}
          class:ac-highlighted={i === highlighted}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => select(s)}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') select(s); }}
        >
          <span class="t-mono">{s.slug}</span> · {s.canonical_name}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .ac { position: relative; }
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
