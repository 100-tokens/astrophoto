<script lang="ts">
  import { tick, untrack } from 'svelte';
  import type { PhotoFilterChip } from '$lib/api/PhotoFilterChip';
  import type { FilterType } from '$lib/api/FilterType';
  import type { FilterSpecs } from '$lib/api/FilterSpecs';
  import { FILTER_TYPE_META, bandwidthLabel } from '$lib/equipment/filter-types';
  import FilterChip from './FilterChip.svelte';
  import BrandModelInput, { type BrandModel } from './BrandModelInput.svelte';
  import FilterSpecsForm from './specs/FilterSpecsForm.svelte';
  import './equipment-create-form.css';

  // Shape returned by the autocomplete endpoint. Catalog v2 adds
  // `brand`, `model`, and `specs_summary` per row — used by the
  // popup to render `<strong>brand</strong> · model` plus a small
  // spec line. `filter_type` + `bandwidth_nm` are still populated
  // server-side for the FilterChip badge fast-path on add().
  interface AutocompleteItem {
    id: string;
    canonical_name: string;
    display_name: string;
    usage_count: number;
    brand?: string;
    model?: string;
    variant?: string | null;
    specs_summary?: string | null;
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
  // popup body is replaced by the catalog v2 saisie-forcée create form:
  // <BrandModelInput> + <FilterSpecsForm> + actions. The user-typed
  // `creatingName` pre-fills `model` (brand and variant stay empty until
  // edited). The actual POST runs in `confirmCreate()`.
  let creatingName = $state<string | null>(null);
  let createBM = $state<BrandModel>({ brand: '', model: '', variant: '' });
  let createSpecs = $state<FilterSpecs>({
    filter_type: null,
    bandwidth_nm: null,
    size: null,
    mounted: null,
    mounted_diameter_mm: null,
    thickness_mm: null,
    peak_transmission_pct: null
  });
  let createBusy = $state(false);
  let createError = $state<string | null>(null);
  let createFormEl = $state<HTMLDivElement | undefined>(undefined);

  // Preview the display_name the backend will regenerate from
  // brand + model + variant. Empty brand → just model (+ variant).
  const displayPreview = $derived(
    [createBM.brand.trim(), createBM.model.trim(), createBM.variant.trim()]
      .filter(Boolean)
      .join(' ')
  );

  // Bandwidth is required only for narrowband-style types — we reuse
  // `showBandwidth` from the chip metadata so the predicate stays in
  // one place.
  const needsBandwidth = $derived(
    !!createSpecs.filter_type && FILTER_TYPE_META[createSpecs.filter_type].showBandwidth
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

  // POST /api/equipment/items to create the canonical row, then push it
  // as a chip. Catalog v2: the body carries brand/model/variant (server
  // regenerates display_name + canonical_name from them) and the typed
  // FilterSpecs payload. We use the response's `display_name` for the
  // chip — that's what's stored in the catalog and what FilterChip will
  // render everywhere else.
  async function postNewItem(): Promise<boolean> {
    const body: Record<string, unknown> = {
      kind: 'filter',
      // display_name kept for back-compat; the backend regenerates it
      // from brand/model/variant when those are non-empty.
      display_name: displayPreview,
      brand: createBM.brand.trim(),
      model: createBM.model.trim(),
      variant: createBM.variant.trim() || null,
      specs: {
        kind: 'filter',
        filter_type: createSpecs.filter_type,
        bandwidth_nm: needsBandwidth ? createSpecs.bandwidth_nm : null,
        mounted_diameter_mm: createSpecs.mounted_diameter_mm,
        thickness_mm: createSpecs.thickness_mm,
        peak_transmission_pct: createSpecs.peak_transmission_pct
      }
    };

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
      filter_type: createSpecs.filter_type,
      bandwidth_nm: needsBandwidth ? createSpecs.bandwidth_nm : null,
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
    // without a round-trip.
    if (!row.id) {
      // Older backends (no id in the autocomplete payload) are no longer
      // a supported path now that the create flow has moved to a typed
      // popover — fail safe by ignoring rather than creating untyped.
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
    // Seed model with the user-typed name; leave brand + variant empty
    // for the user to fill. Auto-splitting on whitespace would lie for
    // multi-word brands ("Sky-Watcher Esprit 100 ED"), so we don't.
    creatingName = name;
    createBM = { brand: '', model: name, variant: '' };
    createSpecs = {
      filter_type: null,
      bandwidth_nm: null,
      size: null,
      mounted: null,
      mounted_diameter_mm: null,
      thickness_mm: null,
      peak_transmission_pct: null
    };
    createError = null;
    // Autofocus the first input (Brand) once the form is in the DOM.
    // The first <input> inside BrandModelInput is the Brand field.
    void tick().then(() => {
      const first = createFormEl?.querySelector<HTMLInputElement>('input');
      first?.focus();
    });
  }

  function cancelCreate() {
    creatingName = null;
    createError = null;
    // Return focus to the chip-input so the user can keep typing.
    inputEl?.focus();
  }

  function validate(): string | null {
    if (!createBM.brand.trim()) {
      // Brand is allowed to be empty (the canonical row can carry
      // brand=""), but UX-wise we want the catalog to be useful — push
      // the user to fill it. Soft-validate by suggesting, not blocking,
      // when the model field looks like a multi-word descriptor that
      // probably contains the brand. Hard requirement: model non-empty.
    }
    if (!createBM.model.trim()) return 'Model is required.';
    if (!createSpecs.filter_type) return 'Pick a filter type.';
    if (needsBandwidth) {
      const bw = createSpecs.bandwidth_nm;
      if (bw === null || !Number.isFinite(bw) || bw <= 0) {
        return 'Bandwidth (nm) is required for this filter type.';
      }
    }
    if (
      createSpecs.mounted_diameter_mm === null ||
      !Number.isFinite(createSpecs.mounted_diameter_mm) ||
      createSpecs.mounted_diameter_mm <= 0
    ) {
      return 'Mounted diameter is required.';
    }
    return null;
  }

  async function confirmCreate() {
    if (creatingName === null) return;
    const err = validate();
    if (err) {
      createError = err;
      return;
    }
    createBusy = true;
    createError = null;
    const ok = await postNewItem();
    createBusy = false;
    if (!ok) {
      createError = 'Could not create the filter item. Try again.';
      return;
    }
    creatingName = null;
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
    // Cancel on Escape; Enter is intentionally NOT bound to confirm —
    // the user often hits Enter inside a number input to commit a digit,
    // and we don't want that to fire the POST. The "Create filter"
    // button is the only confirm path.
    if (e.key === 'Escape') {
      e.preventDefault();
      cancelCreate();
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
        <!-- Catalog v2 saisie-forcée: BrandModelInput on top + the
             canonical FilterSpecsForm. The popup body becomes a real
             create dialog while the user fills the required fields.
             Forces brand/model/filter_type/(bandwidth)/mounted_diameter
             so every new row lands typed and indexable. -->
        <div class="fchip-pop-head">
          <span>NEW FILTER · DETAILS</span>
          <span style="color: var(--fg-faint)">REQUIRED</span>
        </div>
        <div class="ec-create-form" bind:this={createFormEl} onkeydown={onFormKey}>
          <BrandModelInput
            value={createBM}
            disabled={createBusy}
            label={displayPreview || null}
            onChange={(next) => {
              createBM = next;
              // Focus the Brand input on first open. The first
              // <input> rendered by BrandModelInput is the Brand
              // field — we capture it via a small effect below.
            }}
          />
          <FilterSpecsForm
            value={createSpecs}
            disabled={createBusy}
            onChange={(next) => (createSpecs = next)}
          />
          {#if createError}
            <div class="ec-create-error">{createError}</div>
          {/if}
          <div class="ec-create-actions">
            <button
              type="button"
              class="ec-create-btn"
              onclick={cancelCreate}
              disabled={createBusy}
            >
              Cancel
            </button>
            <button
              type="button"
              class="ec-create-btn is-primary"
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
              {@const hasBrand = (f.brand ?? '').trim().length > 0}
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
                <span class="brand-model">
                  {#if hasBrand}
                    <strong>{f.brand}</strong> · {f.model}
                  {:else}
                    {f.display_name}
                  {/if}
                </span>
                <span class="meta">
                  {#if f.specs_summary}
                    {f.specs_summary.toUpperCase()}
                  {:else if meta}
                    {meta.label.toUpperCase()}{#if bw}
                      · {bw.toUpperCase()}{/if}
                  {:else}
                    UNTYPED
                  {/if}
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

<style>
  /* The popup item now renders a brand-model line plus the spec
     summary — give those their own typography (brand-model is
     ui-font, meta stays mono). */
  :global(.fchip-pop-item .brand-model) {
    flex: 1;
    color: var(--fg-secondary);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  :global(.fchip-pop-item .brand-model strong) {
    color: var(--fg-primary);
    font-weight: 600;
  }
</style>
