<script lang="ts">
  import { tick, untrack } from 'svelte';
  import type { PhotoFilterChip } from '$lib/api/PhotoFilterChip';
  import type { FilterType } from '$lib/api/FilterType';
  import { FILTER_TYPE_META, bandwidthLabel } from '$lib/equipment/filter-types';
  import FilterChip from './FilterChip.svelte';

  // Filter types are declared in `$lib/api/FilterType` and the badge/label
  // metadata lives in `$lib/equipment/filter-types`. Iterating the meta map
  // gives us a single source of truth for the select options. `showBandwidth`
  // doubles as our "narrowband-style → bandwidth_nm required" predicate.
  const FILTER_TYPE_OPTIONS = Object.entries(FILTER_TYPE_META).map(([value, meta]) => ({
    value: value as FilterType,
    label: meta.label
  }));

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
  // Count of autocomplete rows for the current query that are not yet
  // picked, BEFORE the slice(0, 8) display cap. The popup header renders
  // "{matches.length} OF {available}". Note: the server returns query-
  // filtered (or "popular") results, not the full catalog, so this is
  // an upper bound on "currently shown" / "matching the query", not
  // "total catalog minus picked".
  let available = $state(0);
  let inputEl = $state<HTMLInputElement | undefined>(undefined);

  // Create-with-specs sub-form state. When `creatingName` is non-null the
  // popup body is replaced by a small spec form prompting for the
  // mandatory `filter_type` (and `bandwidth_nm` when narrowband) before
  // posting to /api/equipment/items. This enforces issue #6 of the
  // catalog coherence audit: every new filter item must carry typed
  // specs so the chip renders with its proper badge instead of "?".
  let creatingName = $state<string | null>(null);
  let createFilterType = $state<FilterType | ''>('');
  let createBandwidth = $state<string>('');
  let createBusy = $state(false);
  let createError = $state<string | null>(null);
  let filterTypeSelectEl = $state<HTMLSelectElement | undefined>(undefined);

  // Bandwidth is required only for narrowband-style types — we reuse
  // `showBandwidth` from the chip metadata so the predicate stays in
  // one place.
  const needsBandwidth = $derived(
    !!createFilterType && FILTER_TYPE_META[createFilterType].showBandwidth
  );

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
        available = 0;
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
        const dedup = data.items.filter(
          (row) =>
            !currentItems.some(
              (i) => i.display_name.toLowerCase() === row.display_name.toLowerCase()
            )
        );
        untrack(() => {
          available = dedup.length;
          matches = dedup.slice(0, 8);
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
  // When `specs` is provided the chip is pushed with its typed badge so the
  // FilterChip UI renders the proper code instead of "?".
  async function resolveAndAdd(
    displayName: string,
    specs: { filter_type: FilterType; bandwidth_nm: number | null } | null = null
  ): Promise<boolean> {
    const body: Record<string, unknown> = { kind: 'filter', display_name: displayName };
    if (specs !== null) {
      body.specs = {
        kind: 'filter',
        filter_type: specs.filter_type,
        bandwidth_nm: specs.bandwidth_nm
      };
    }

    let res: Response;
    try {
      res = await fetch('/api/equipment/items', {
        method: 'POST',
        credentials: 'include',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(body)
      });
    } catch {
      return false;
    }
    if (!res.ok) return false;

    let item: { id: string; display_name: string } & Record<string, unknown>;
    try {
      item = await res.json();
    } catch {
      return false;
    }

    const chip: PhotoFilterChip = {
      id: item.id,
      display_name: item.display_name,
      filter_type: specs ? specs.filter_type : null,
      bandwidth_nm: specs ? specs.bandwidth_nm : null,
      position: items.length
    };
    const next = [...items, chip];
    items = next;
    onChange(next);
    query = '';
    focusIdx = 0;
    return true;
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
    // Open the spec sub-form. The actual POST is deferred to
    // `confirmCreate()` once the user picks a filter_type (and bandwidth
    // when relevant). We do NOT fall back to a typeless create — that
    // would re-introduce the "?" badge issue we're fixing.
    creatingName = name;
    createFilterType = '';
    createBandwidth = '';
    createError = null;
    // Autofocus the type select once the form is in the DOM.
    void tick().then(() => filterTypeSelectEl?.focus());
  }

  function cancelCreate() {
    creatingName = null;
    createFilterType = '';
    createBandwidth = '';
    createError = null;
    // Return focus to the chip-input so the user can keep typing.
    inputEl?.focus();
  }

  async function confirmCreate() {
    if (creatingName === null) return;
    if (!createFilterType) {
      createError = 'Pick a filter type.';
      return;
    }

    let bandwidth: number | null = null;
    if (FILTER_TYPE_META[createFilterType].showBandwidth) {
      const raw = createBandwidth.trim();
      if (!raw) {
        createError = 'Bandwidth (nm) is required for this filter type.';
        return;
      }
      const parsed = Number(raw);
      if (!Number.isFinite(parsed) || parsed <= 0) {
        createError = 'Bandwidth must be a positive number.';
        return;
      }
      bandwidth = parsed;
    }

    createBusy = true;
    createError = null;
    const ok = await resolveAndAdd(creatingName, {
      filter_type: createFilterType,
      bandwidth_nm: bandwidth
    });
    createBusy = false;

    if (!ok) {
      createError = 'Could not create the filter item. Try again.';
      return;
    }

    // Reset and return focus to the chip-input for the next entry.
    creatingName = null;
    createFilterType = '';
    createBandwidth = '';
    inputEl?.focus();
  }

  function onKey(e: KeyboardEvent) {
    // While the spec sub-form is open the chip-input is no longer the
    // active surface — let the sub-form own its own keys.
    if (creatingName !== null) return;

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

  function onFormKey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      cancelCreate();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      void confirmCreate();
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
    onblur={() =>
      setTimeout(() => {
        // Keep the popup mounted while the spec sub-form is active —
        // focus is intentionally moving from the input to the form.
        if (creatingName === null) open = false;
      }, 150)}
    onkeydown={onKey}
    placeholder={items.length ? '' : placeholder}
    aria-label={placeholder}
  />

  {#if open}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="fchip-pop">
      {#if creatingName !== null}
        <!-- Spec sub-form: forces filter_type (and bandwidth_nm when
             narrowband) before the POST so the new chip renders typed.
             Without this the row would land in `equipment_items` without
             a `filter_specs` partner and the FilterChip would show "?". -->
        <div class="fchip-pop-head">
          <span>NEW FILTER · DETAILS</span>
          <span style="color: var(--fg-faint)">REQUIRED</span>
        </div>
        <div class="fchip-create-form" onkeydown={onFormKey}>
          <div class="fchip-create-row">
            <span class="fchip-create-label">Name</span>
            <span class="fchip-create-name">{creatingName}</span>
          </div>
          <label class="fchip-create-row">
            <span class="fchip-create-label">Type</span>
            <select
              bind:this={filterTypeSelectEl}
              class="fchip-create-input"
              bind:value={createFilterType}
              disabled={createBusy}
            >
              <option value="" disabled>Pick a filter type…</option>
              {#each FILTER_TYPE_OPTIONS as opt (opt.value)}
                <option value={opt.value}>{opt.label}</option>
              {/each}
            </select>
          </label>
          <label class="fchip-create-row">
            <span class="fchip-create-label">
              Bandwidth
              {#if !needsBandwidth && createFilterType}
                <span class="fchip-create-hint">n/a</span>
              {/if}
            </span>
            <span class="fchip-create-bw">
              <input
                class="fchip-create-input"
                type="number"
                min="0"
                step="0.1"
                inputmode="decimal"
                placeholder={needsBandwidth ? 'e.g. 6' : '—'}
                bind:value={createBandwidth}
                disabled={!needsBandwidth || createBusy}
              />
              <span class="fchip-create-unit">nm</span>
            </span>
          </label>
          {#if createError}
            <div class="fchip-create-error">{createError}</div>
          {/if}
          <div class="fchip-create-actions">
            <button
              type="button"
              class="fchip-create-btn"
              onclick={cancelCreate}
              disabled={createBusy}
            >
              Cancel
            </button>
            <button
              type="button"
              class="fchip-create-btn is-primary"
              onclick={() => {
                void confirmCreate();
              }}
              disabled={createBusy}
            >
              {createBusy ? 'Creating…' : 'Create filter'}
            </button>
          </div>
        </div>
      {:else}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div onmousedown={(e) => e.preventDefault()}>
          <div class="fchip-pop-head">
            <span>{query ? `MATCHES "${query}"` : 'POPULAR FILTERS'}</span>
            {#if available > 0}
              <span style="color: var(--fg-faint)">{matches.length} OF {available}</span>
            {/if}
          </div>
          <div class="fchip-pop-list">
            {#if matches.length === 0}
              <div
                style="padding: 14px; color: var(--fg-muted); font-size: 12px; font-family: var(--font-mono)"
              >
                {#if query.trim()}
                  No matches — press <span style="color: var(--accent)">↵ Enter</span> to create "<span
                    style="color: var(--fg-primary)">{query.trim()}</span
                  >"
                {:else}
                  No filters yet — type to search or create one.
                {/if}
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
                    id: f.id ?? '',
                    display_name: f.display_name,
                    filter_type: f.filter_type ?? null,
                    bandwidth_nm: f.bandwidth_nm ?? null,
                    position: 0
                  }}
                  compact
                />
                <span class="meta">
                  {#if meta}{meta.label.toUpperCase()}{#if bw}
                      · {bw.toUpperCase()}{/if}{:else}UNTYPED{/if}
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
  {/if}
</div>
