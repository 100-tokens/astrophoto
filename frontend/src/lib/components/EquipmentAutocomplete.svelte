<script lang="ts">
  type EquipmentKind = 'telescope' | 'camera' | 'mount' | 'filter' | 'guiding';

  interface Props {
    name: string;
    kind: EquipmentKind;
    value?: string;
    api?: string;
  }

  let {
    name,
    kind,
    value = $bindable(''),
    api = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? ''
  }: Props = $props();

  type Item = { canonical_name: string; display_name: string; usage_count: number };
  let items = $state<Item[]>([]);
  let highlighted = $state(-1);
  let lastSelected = $state('');

  // Stale-response guard — same reqId pattern as HandlePicker.
  let reqId = 0;

  $effect(() => {
    // Suppress re-fetch when the user just selected a suggestion.
    if (!value || value === lastSelected) {
      items = [];
      highlighted = -1;
      return;
    }
    const myId = ++reqId;
    const t = setTimeout(async () => {
      try {
        const r = await fetch(
          `${api}/api/equipment/autocomplete?kind=${encodeURIComponent(kind)}&q=${encodeURIComponent(value)}`
        );
        if (r.ok && myId === reqId) {
          items = (await r.json()).items;
          highlighted = -1;
        }
      } catch {
        if (myId === reqId) items = [];
      }
    }, 200);
    return () => clearTimeout(t);
  });

  function select(item: Item) {
    lastSelected = item.display_name;
    value = item.display_name;
    items = [];
    highlighted = -1;
  }

  function onKeydown(e: KeyboardEvent) {
    if (!items.length) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      highlighted = Math.min(highlighted + 1, items.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      highlighted = Math.max(-1, highlighted - 1);
    } else if (e.key === 'Enter' && highlighted >= 0) {
      e.preventDefault();
      const item = items[highlighted];
      if (item) select(item);
    } else if (e.key === 'Escape') {
      items = [];
      highlighted = -1;
    }
  }

  function onBlur() {
    // Small delay so onmousedown={e.preventDefault()} on <li> can fire first.
    setTimeout(() => {
      items = [];
      highlighted = -1;
    }, 120);
  }
</script>

<label class="t-label" for={name}>{kind.toUpperCase()}</label>
<div class="ac">
  <input
    id={name}
    {name}
    bind:value
    class="input input-mono"
    onkeydown={onKeydown}
    onblur={onBlur}
    autocomplete="off"
    spellcheck="false"
    aria-autocomplete="list"
    aria-expanded={items.length > 0}
  />
  {#if items.length}
    <ul class="ac-list card" role="listbox">
      {#each items as item, i}
        <!-- onmousedown prevents blur from firing before click, keeping focus intact.
             Keyboard nav (↑↓ Enter Esc) on the <input> above handles all keyboard cases. -->
        <li
          role="option"
          aria-selected={i === highlighted}
          class:ac-highlighted={i === highlighted}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => select(item)}
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') select(item);
          }}
        >
          {item.display_name}
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
    max-height: 200px;
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
