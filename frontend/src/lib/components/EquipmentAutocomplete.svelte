<script lang="ts">
  type EquipmentKind = 'telescope' | 'camera' | 'mount' | 'filter' | 'focal_modifier' | 'guiding';

  type Committed = { id: string; display_name: string } | null;

  interface Props {
    name: string;
    kind: EquipmentKind;
    value?: string;
    api?: string;
    /**
     * Called when the user finalizes a value: clicks a suggestion OR
     * blurs the input with a non-empty free-typed value. The component
     * does a `POST /api/equipment/items` resolve-or-create against the
     * current `kind` and the typed/selected `display_name`, then emits
     * the resulting canonical `{ id, display_name }`. Skipped when
     * value is empty (emits `null`) and when kind is `guiding` (no
     * canonical-items dictionary for guiding by design).
     */
    onCommit?: (committed: Committed) => void;
    /**
     * Override the default label (which is the upper-cased kind, e.g.
     * 'TELESCOPE'). Set to null to suppress the label entirely — useful
     * when a parent component already renders its own label.
     */
    label?: string | null;
  }

  let {
    name,
    kind,
    value = $bindable(''),
    api = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '',
    onCommit,
    label
  }: Props = $props();

  type Item = { canonical_name: string; display_name: string; usage_count: number };
  let items = $state<Item[]>([]);
  let highlighted = $state(-1);
  let lastSelected = $state('');
  // Tracks the last text that was committed to avoid no-op blur round-trips.
  let lastCommitted = $state(value ?? '');

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

  async function commit(displayName: string) {
    if (!onCommit) return;
    const trimmed = displayName.trim();
    if (!trimmed) {
      onCommit(null);
      return;
    }
    // Skip no-op blur (nothing changed since last commit).
    if (trimmed === lastCommitted.trim()) return;
    // 'guiding' is intentionally non-canonical: setup form treats it as
    // free text and never asks the autocomplete to resolve it.
    if (kind === 'guiding') {
      return;
    }
    try {
      const r = await fetch(`${api}/api/equipment/items`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include',
        body: JSON.stringify({ kind, display_name: trimmed })
      });
      if (r.ok) {
        const row = await r.json();
        lastCommitted = row.display_name;
        onCommit({ id: row.id, display_name: row.display_name });
      }
    } catch {
      // Silent: the consumer can decide what to do if no commit fires.
    }
  }

  function select(item: Item) {
    lastSelected = item.display_name;
    lastCommitted = item.display_name;
    value = item.display_name;
    items = [];
    highlighted = -1;
    commit(item.display_name);
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
    commit(value);
  }
</script>

{#if label !== null}
  <label class="t-label" for={name}>{label ?? kind.toUpperCase()}</label>
{/if}
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
    aria-label={label ?? kind}
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
