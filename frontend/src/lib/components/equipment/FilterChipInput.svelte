<script lang="ts">
  import { untrack } from 'svelte';
  import type { PhotoFilterChip } from '$lib/api/PhotoFilterChip';
  import type { FilterType } from '$lib/api/FilterType';
  import { FILTER_TYPE_META, bandwidthLabel } from '$lib/equipment/filter-types';
  import FilterChip from './FilterChip.svelte';

  // Shape returned by the autocomplete endpoint. `id`, `filter_type`,
  // `bandwidth_nm` are populated for kind=filter rows so the popup chip
  // can render typed without a follow-up GET, and add() can skip the
  // resolve-or-create POST when the row already has a canonical id.
  interface AutocompleteItem {
    id: string;
    canonical_name: string;
    display_name: string;
    usage_count: number;
    filter_type?: FilterType | null;
    bandwidth_nm?: number | null;
  }

  type Props = {
    value: PhotoFilterChip[];
    orphans?: string[];
    startOpen?: boolean;
    placeholder?: string;
    onChange: (next: PhotoFilterChip[]) => void;
  };

  let {
    value,
    orphans = [],
    startOpen = false,
    placeholder = 'Search filters…',
    onChange
  }: Props = $props();

  // Initialize with untrack so Svelte doesn't warn about prop captures.
  // Effects below handle ongoing synchronisation.
  let items = $state<PhotoFilterChip[]>(untrack(() => [...value]));
  let query = $state('');
  let open = $state(untrack(() => startOpen));
  let focusIdx = $state(0);
  let dragId = $state<string | null>(null);
  let matches = $state<AutocompleteItem[]>([]);
  let inputEl = $state<HTMLInputElement | undefined>(undefined);

  // Sync items when the parent passes a fresh value reference.
  $effect(() => {
    const incoming = value;
    untrack(() => {
      items = [...incoming];
    });
  });

  // Autocomplete fetch: re-run when query or open changes
  $effect(() => {
    const isOpen = open;
    const q = query.trim();
    if (!isOpen) {
      untrack(() => {
        matches = [];
      });
      return;
    }

    const currentItems = untrack(() => items);
    const controller = new AbortController();

    fetch(`/api/equipment/autocomplete?kind=filter&q=${encodeURIComponent(q)}`, {
      credentials: 'include',
      signal: controller.signal
    })
      .then((r) => (r.ok ? r.json() : Promise.reject(r)))
      .then((data: { items: AutocompleteItem[] }) => {
        const filtered = data.items
          .filter(
            (row) =>
              !currentItems.some(
                (i) => i.display_name.toLowerCase() === row.display_name.toLowerCase()
              )
          )
          .slice(0, 8);
        untrack(() => {
          matches = filtered;
          focusIdx = 0;
        });
      })
      .catch(() => {
        // Aborted or error — leave current matches unchanged
      });

    return () => controller.abort();
  });

  // POST /api/equipment/items to resolve-or-create, then add as chip.
  // The autocomplete endpoint does not return item ids, so every "add" goes
  // through this resolve-or-create call. For existing items it is idempotent.
  async function resolveAndAdd(displayName: string) {
    let res: Response;
    try {
      res = await fetch('/api/equipment/items', {
        method: 'POST',
        credentials: 'include',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ kind: 'filter', display_name: displayName })
      });
    } catch {
      return;
    }
    if (!res.ok) return;

    let item: { id: string; display_name: string } & Record<string, unknown>;
    try {
      item = await res.json();
    } catch {
      return;
    }

    const chip: PhotoFilterChip = {
      id: item.id,
      display_name: item.display_name,
      filter_type: null,
      bandwidth_nm: null,
      position: items.length
    };
    const next = [...items, chip];
    items = next;
    onChange(next);
    query = '';
    focusIdx = 0;
  }

  function add(row: AutocompleteItem) {
    // Fast path: the autocomplete row now carries the canonical id (and
    // its typed specs when kind='filter'), so we can build the chip
    // without a round-trip. Fall back to resolve-or-create only if the
    // server somehow returned a row without an id (back-compat with an
    // older backend).
    if (!row.id) {
      resolveAndAdd(row.display_name);
      return;
    }
    const chip: PhotoFilterChip = {
      id: row.id,
      display_name: row.display_name,
      filter_type: row.filter_type ?? null,
      bandwidth_nm: row.bandwidth_nm ?? null,
      position: items.length
    };
    const next = [...items, chip];
    items = next;
    onChange(next);
    query = '';
    focusIdx = 0;
  }

  function remove(id: string) {
    const next = items.filter((f) => f.id !== id);
    items = next;
    onChange(next);
  }

  function reorder(srcId: string, beforeId: string | null) {
    if (srcId === beforeId) return;
    const src = items.find((f) => f.id === srcId);
    if (!src) return;
    const rest = items.filter((f) => f.id !== srcId);
    const idx = beforeId == null ? rest.length : rest.findIndex((f) => f.id === beforeId);
    const next = [...rest.slice(0, idx), src, ...rest.slice(idx)];
    items = next;
    onChange(next);
  }

  function createNew() {
    const name = query.trim();
    if (!name) return;
    resolveAndAdd(name);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      focusIdx = Math.min(matches.length - 1, focusIdx + 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      focusIdx = Math.max(0, focusIdx - 1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const focused = matches[focusIdx];
      if (focused !== undefined) {
        add(focused);
      } else if (query.trim()) {
        createNew();
      }
    } else if (e.key === 'Backspace' && !query && items.length > 0) {
      const last = items[items.length - 1];
      if (last !== undefined) remove(last.id);
    } else if (e.key === 'Escape') {
      open = false;
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="fchip-input"
  onclick={() => {
    inputEl?.focus();
    open = true;
  }}
>
  {#each items as f (f.id)}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <span
      draggable="true"
      ondragstart={(e) => {
        dragId = f.id;
        e.dataTransfer!.effectAllowed = 'move';
      }}
      ondragover={(e) => {
        e.preventDefault();
      }}
      ondrop={(e) => {
        e.preventDefault();
        if (dragId) {
          reorder(dragId, f.id);
          dragId = null;
        }
      }}
      ondragend={() => {
        dragId = null;
      }}
      style="display:inline-flex"
    >
      <FilterChip
        filter={f}
        draggable
        removable
        dragging={dragId === f.id}
        onRemove={() => remove(f.id)}
      />
    </span>
  {/each}

  {#each orphans as tok, i (i)}
    <span class="fchip-orphan" title="Legacy text filter — no catalog match yet">
      <span class="lbl">legacy</span>{tok}
    </span>
  {/each}

  <input
    bind:this={inputEl}
    value={query}
    oninput={(e) => {
      query = (e.target as HTMLInputElement).value;
      open = true;
      focusIdx = 0;
    }}
    onfocus={() => (open = true)}
    onblur={() => setTimeout(() => (open = false), 150)}
    onkeydown={onKey}
    placeholder={items.length ? '' : placeholder}
  />

  {#if open}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="fchip-pop" onmousedown={(e) => e.preventDefault()}>
      <div class="fchip-pop-head">
        <span>{query ? `MATCHES "${query}"` : 'POPULAR FILTERS'}</span>
        <span style="color: var(--fg-faint)">{matches.length} RESULTS</span>
      </div>
      <div class="fchip-pop-list">
        {#if matches.length === 0}
          <div
            style="padding: 14px; color: var(--fg-muted); font-size: 12px; font-family: var(--font-mono)"
          >
            No matches. Press <span style="color: var(--accent)">↵ Enter</span> to create a new filter
            item.
          </div>
        {/if}
        {#each matches as f, i (f.display_name)}
          {@const meta = f.filter_type ? FILTER_TYPE_META[f.filter_type] : null}
          {@const bw = bandwidthLabel(f)}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class={'fchip-pop-item' + (i === focusIdx ? ' is-focus' : '')}
            onclick={() => add(f)}
            onmouseenter={() => (focusIdx = i)}
          >
            <FilterChip
              filter={{
                id: '',
                display_name: f.display_name,
                filter_type: null,
                bandwidth_nm: null,
                position: 0
              }}
              compact
            />
            <span class="meta">
              {#if meta}{meta.label.toUpperCase()}{#if bw} · {bw.toUpperCase()}{/if}{:else}UNTYPED{/if}
            </span>
            <span class="usage">{f.usage_count.toLocaleString()} PHOTOS</span>
          </div>
        {/each}
      </div>
      {#if query.trim()}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="fchip-pop-create" onclick={createNew}>
          <svg
            width="12"
            height="12"
            viewBox="0 0 12 12"
            fill="none"
            stroke="currentColor"
            stroke-width="1.4"
          >
            <path d="M6 2v8M2 6h8" stroke-linecap="round" />
          </svg>
          <span>Create new · "<strong style="color: var(--fg-primary)">{query}</strong>"</span>
          <span class="kbd">↵ ENTER</span>
        </div>
      {/if}
    </div>
  {/if}
</div>
