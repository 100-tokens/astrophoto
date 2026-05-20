<script lang="ts">
  import { tick } from 'svelte';
  import BrandModelInput, {
    type BrandModel
  } from '$lib/components/equipment/BrandModelInput.svelte';
  import TelescopeSpecsForm from '$lib/components/equipment/specs/TelescopeSpecsForm.svelte';
  import CameraSpecsForm from '$lib/components/equipment/specs/CameraSpecsForm.svelte';
  import MountSpecsForm from '$lib/components/equipment/specs/MountSpecsForm.svelte';
  import FocalModifierSpecsForm from '$lib/components/equipment/specs/FocalModifierSpecsForm.svelte';
  import GuidingSpecsForm from '$lib/components/equipment/specs/GuidingSpecsForm.svelte';
  import type { TelescopeSpecs } from '$lib/api/TelescopeSpecs';
  import type { CameraSpecs } from '$lib/api/CameraSpecs';
  import type { MountSpecs } from '$lib/api/MountSpecs';
  import type { FocalModifierSpecs } from '$lib/api/FocalModifierSpecs';
  import type { GuidingSpecs } from '$lib/api/GuidingSpecs';
  import '$lib/components/equipment/equipment-create-form.css';

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

  // Catalog v2 (migration 0022): autocomplete rows now carry brand /
  // model / variant + a short specs_summary computed server-side. The
  // dropdown renders `<strong>brand</strong> · model` when brand is
  // populated, plus the spec summary line. Legacy/freetext rows
  // (brand = "") fall back to plain display_name.
  type Item = {
    id?: string;
    canonical_name: string;
    display_name: string;
    usage_count: number;
    brand?: string;
    model?: string;
    variant?: string | null;
    specs_summary?: string | null;
  };
  let items = $state<Item[]>([]);
  let highlighted = $state(-1);
  let lastSelected = $state('');
  // Tracks the last text that was committed to avoid no-op blur round-trips.
  let lastCommitted = $state(value ?? '');

  // Stale-response guard — same reqId pattern as HandlePicker.
  let reqId = 0;

  // ── Create-with-specs popover state ─────────────────────────────────
  //
  // Catalog v2 saisie-forcée: when the user clicks the "Create new" row
  // in the dropdown (or hits Enter with no suggestion focused and no
  // exact-name match) we open a popover overlaying the input with
  // <BrandModelInput> + the per-kind <*SpecsForm>. Confirming POSTs to
  // /api/equipment/items with the structured fields; the response's
  // canonical display_name becomes the new input value.
  //
  // The popover is fully internal to this component — it works whether
  // or not `onCommit` is wired (e.g. VerifyPane uses bind:value only and
  // never reads onCommit; the photo PUT later freetext-upserts).
  let creating = $state(false);
  let createBM = $state<BrandModel>({ brand: '', model: '', variant: '' });
  let createBusy = $state(false);
  let createError = $state<string | null>(null);
  let createFormEl = $state<HTMLDivElement | undefined>(undefined);

  // Per-kind spec state. Only the field matching the current `kind`
  // is ever read by postNewItem(); the others stay at their defaults
  // so the spec sub-form components have a stable starting value
  // (they're controlled — `value` can't be undefined).
  let telescopeSpecs = $state<TelescopeSpecs>({
    design: null,
    aperture_mm: null,
    focal_length_mm: null,
    focal_ratio_f: null,
    self_weight_kg: null,
    optical_length_mm: null,
    backfocus_mm: null
  });
  let cameraSpecs = $state<CameraSpecs>({
    sensor_type: null,
    color_type: null,
    cooled: null,
    sensor_model: null,
    pixel_size_um: null,
    sensor_width_px: null,
    sensor_height_px: null,
    self_weight_g: null,
    full_well_capacity_e: null,
    read_noise_e: null,
    mount_thread: null,
    backfocus_mm: null
  });
  let mountSpecs = $state<MountSpecs>({
    mount_type: null,
    payload_kg: null,
    goto: null,
    self_weight_kg: null,
    periodic_error_arcsec: null,
    tripod_included: null,
    control_protocol: null
  });
  let focalModSpecs = $state<FocalModifierSpecs>({
    modifier_type: null,
    factor: null,
    self_weight_g: null,
    backfocus_mm: null,
    image_circle_mm: null
  });
  let guidingSpecs = $state<GuidingSpecs>({
    setup_kind: null,
    guide_focal_mm: null,
    guide_aperture_mm: null,
    guide_camera: null
  });

  // Preview the display_name the backend will regenerate from
  // brand + model + variant. Empty brand → just model (+ variant).
  const displayPreview = $derived(
    [createBM.brand.trim(), createBM.model.trim(), createBM.variant.trim()]
      .filter(Boolean)
      .join(' ')
  );

  $effect(() => {
    // Suppress re-fetch when the user just selected a suggestion or the
    // create popover is open (the input is hidden behind it).
    if (creating) {
      items = [];
      highlighted = -1;
      return;
    }
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

  // Whether any visible suggestion exactly matches the typed text. We
  // only show the "Create new" footer when there's no exact match —
  // typing a real catalog name shouldn't add a "Create new" row.
  const hasExactMatch = $derived(
    items.some((i) => i.display_name.trim().toLowerCase() === value.trim().toLowerCase())
  );

  function openCreate() {
    if (!value.trim()) return;
    // Seed model with the typed value; brand + variant empty for the
    // user to fill. Auto-splitting on whitespace would lie for
    // multi-word brands.
    createBM = { brand: '', model: value.trim(), variant: '' };
    createError = null;
    creating = true;
    // Autofocus the first input (Brand) once the form is in the DOM.
    void tick().then(() => {
      const first = createFormEl?.querySelector<HTMLInputElement>('input');
      first?.focus();
    });
  }

  function cancelCreate() {
    creating = false;
    createError = null;
  }

  function validate(): string | null {
    if (!createBM.model.trim()) return 'Model is required.';

    // Per-kind required spec field checks. Mirrors the "above the fold"
    // REQUIRED fields each *SpecsForm renders with the accent dot.
    if (kind === 'telescope') {
      const s = telescopeSpecs;
      if (!s.design) return 'Design is required.';
      if (!s.aperture_mm) return 'Aperture is required.';
      if (!s.focal_length_mm) return 'Focal length is required.';
      if (!s.self_weight_kg) return 'Self weight is required.';
    } else if (kind === 'camera') {
      const s = cameraSpecs;
      if (!s.sensor_type) return 'Sensor type is required.';
      if (!s.color_type) return 'Color type is required.';
      if (s.cooled === null) return 'Cooled flag is required.';
      if (!s.sensor_model || !s.sensor_model.trim()) return 'Sensor model is required.';
      if (!s.pixel_size_um) return 'Pixel size is required.';
      if (!s.sensor_width_px) return 'Sensor width is required.';
      if (!s.sensor_height_px) return 'Sensor height is required.';
      if (!s.self_weight_g) return 'Self weight is required.';
    } else if (kind === 'mount') {
      const s = mountSpecs;
      if (!s.mount_type) return 'Mount type is required.';
      if (!s.payload_kg) return 'Payload is required.';
      if (!s.self_weight_kg) return 'Self weight is required.';
      if (s.goto === null) return 'GoTo flag is required.';
    } else if (kind === 'focal_modifier') {
      const s = focalModSpecs;
      if (!s.modifier_type) return 'Modifier type is required.';
      if (!s.factor) return 'Factor is required.';
      if (!s.self_weight_g) return 'Self weight is required.';
    } else if (kind === 'guiding') {
      const s = guidingSpecs;
      if (!s.setup_kind) return 'Setup kind is required.';
      if (!s.guide_focal_mm) return 'Guide focal length is required.';
      if (!s.guide_aperture_mm) return 'Guide aperture is required.';
    }
    return null;
  }

  function specsPayload(): Record<string, unknown> {
    switch (kind) {
      case 'telescope': {
        const { focal_ratio_f: _ignored, ...rest } = telescopeSpecs;
        void _ignored;
        return { kind, ...rest };
      }
      case 'camera':
        return { kind, ...cameraSpecs };
      case 'mount':
        return { kind, ...mountSpecs };
      case 'focal_modifier':
        return { kind, ...focalModSpecs };
      case 'guiding':
        return { kind, ...guidingSpecs };
      default:
        // 'filter' is handled by FilterChipInput, not this component.
        return { kind };
    }
  }

  async function confirmCreate() {
    const err = validate();
    if (err) {
      createError = err;
      return;
    }
    createBusy = true;
    createError = null;
    try {
      const body = {
        kind,
        // display_name kept for back-compat; the backend regenerates
        // it from brand/model/variant when those are non-empty.
        display_name: displayPreview,
        brand: createBM.brand.trim(),
        model: createBM.model.trim(),
        variant: createBM.variant.trim() || null,
        specs: specsPayload()
      };
      const r = await fetch(`${api}/api/equipment/items`, {
        method: 'POST',
        credentials: 'include',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(body)
      });
      if (!r.ok) {
        createError = 'Could not create the item. Try again.';
        return;
      }
      const row: { id: string; display_name: string } = await r.json();
      // Adopt the canonical display_name as the input value so it
      // matches what the catalog will render going forward.
      value = row.display_name;
      lastSelected = row.display_name;
      lastCommitted = row.display_name;
      creating = false;
      // Notify the parent if it cares about the committed id/name pair.
      if (onCommit) onCommit({ id: row.id, display_name: row.display_name });
    } catch {
      createError = 'Network error — try again.';
    } finally {
      createBusy = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
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
    } else if (e.key === 'Enter' && value.trim() && !hasExactMatch) {
      // No suggestion focused + typed text doesn't exactly match any
      // suggestion → open the create popover. 'guiding' stays freetext
      // when the dictionary route is off — but the popover still works
      // because guiding_specs exists (catalog v2).
      e.preventDefault();
      openCreate();
    } else if (e.key === 'Escape') {
      items = [];
      highlighted = -1;
    }
  }

  function onFormKey(e: KeyboardEvent) {
    // Cancel on Escape; Enter intentionally NOT bound to confirm —
    // the user often hits Enter inside a number input to commit a
    // digit and we don't want that to fire the POST.
    if (e.key === 'Escape') {
      e.preventDefault();
      cancelCreate();
    }
  }

  function onBlur() {
    // Small delay so onmousedown={e.preventDefault()} on <li> can fire first.
    // Don't tear the list down while the create popover is open.
    setTimeout(() => {
      if (creating) return;
      items = [];
      highlighted = -1;
    }, 120);
    if (!creating) commit(value);
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
  {#if items.length || (value.trim() && !hasExactMatch && !creating)}
    <ul class="ac-list card" role="listbox">
      {#each items as item, i (item.canonical_name)}
        {@const hasBrand = (item.brand ?? '').trim().length > 0}
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
          <div class="ac-line-1">
            {#if hasBrand}
              <strong>{item.brand}</strong> · {item.model}
            {:else}
              {item.display_name}
            {/if}
          </div>
          {#if item.specs_summary}
            <div class="ac-line-2">{item.specs_summary}</div>
          {/if}
        </li>
      {/each}
      {#if value.trim() && !hasExactMatch && kind !== 'guiding'}
        <!-- "Create new — <typed>" footer row. Click opens the typed
             create popover. Stays hidden for `guiding` because the
             current verify-form path treats guiding as freetext. -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <li
          class="ac-create"
          onmousedown={(e) => e.preventDefault()}
          onclick={openCreate}
          role="option"
          aria-selected="false"
        >
          <span class="ac-create-plus">+</span>
          Create new · "<strong>{value.trim()}</strong>"
          <span class="ac-create-kbd">↵</span>
        </li>
      {/if}
    </ul>
  {/if}

  {#if creating}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="ac-create-pop" bind:this={createFormEl} onkeydown={onFormKey}>
      <div class="ac-create-pop-head">
        <span>NEW {kind.toUpperCase().replace('_', ' ')} · DETAILS</span>
        <span style="color: var(--fg-faint)">REQUIRED</span>
      </div>
      <div class="ec-create-form">
        <BrandModelInput
          value={createBM}
          disabled={createBusy}
          label={displayPreview || null}
          onChange={(next) => (createBM = next)}
        />
        {#if kind === 'telescope'}
          <TelescopeSpecsForm
            value={telescopeSpecs}
            disabled={createBusy}
            onChange={(next) => (telescopeSpecs = next)}
          />
        {:else if kind === 'camera'}
          <CameraSpecsForm
            value={cameraSpecs}
            disabled={createBusy}
            onChange={(next) => (cameraSpecs = next)}
          />
        {:else if kind === 'mount'}
          <MountSpecsForm
            value={mountSpecs}
            disabled={createBusy}
            onChange={(next) => (mountSpecs = next)}
          />
        {:else if kind === 'focal_modifier'}
          <FocalModifierSpecsForm
            value={focalModSpecs}
            disabled={createBusy}
            onChange={(next) => (focalModSpecs = next)}
          />
        {:else if kind === 'guiding'}
          <GuidingSpecsForm
            value={guidingSpecs}
            disabled={createBusy}
            onChange={(next) => (guidingSpecs = next)}
          />
        {/if}
        {#if createError}
          <div class="ec-create-error">{createError}</div>
        {/if}
        <div class="ec-create-actions">
          <button type="button" class="ec-create-btn" onclick={cancelCreate} disabled={createBusy}>
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
            {createBusy ? 'Creating…' : 'Create item'}
          </button>
        </div>
      </div>
    </div>
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
    max-height: 280px;
    overflow-y: auto;
    z-index: 10;
  }
  .ac-list li {
    padding: 6px 12px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .ac-list li:hover,
  .ac-highlighted {
    background: var(--bg-elevated);
  }
  .ac-line-1 {
    color: var(--fg-primary);
    font-size: 13px;
  }
  .ac-line-1 strong {
    font-weight: 600;
    color: var(--fg-primary);
  }
  .ac-line-2 {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
    letter-spacing: 0.04em;
  }

  /* "Create new — <typed>" footer row inside the dropdown. */
  .ac-create {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: var(--s-2);
    padding: 10px 12px;
    border-top: 1px solid var(--border-subtle);
    font-family: var(--font-mono);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--accent);
    cursor: pointer;
  }
  .ac-create:hover {
    background: var(--bg-elevated);
  }
  .ac-create-plus {
    width: 14px;
    height: 14px;
    display: inline-grid;
    place-items: center;
    border: 1px solid currentColor;
    border-radius: var(--r-sm);
    font-size: 12px;
    line-height: 1;
  }
  .ac-create-kbd {
    margin-left: auto;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--fg-faint);
    border: 1px solid var(--border-default);
    padding: 1px 5px;
    border-radius: 1px;
  }
  .ac-create strong {
    color: var(--fg-primary);
    font-weight: 600;
  }

  /* Popover that replaces the suggestion list once Create is clicked.
     Same surface, contains the BrandModelInput + the per-kind spec
     sub-form + actions. */
  .ac-create-pop {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 4px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-default);
    border-radius: var(--r-sm);
    box-shadow: var(--shadow-lg);
    z-index: 20;
    overflow: hidden;
    max-height: 70vh;
    overflow-y: auto;
  }
  .ac-create-pop-head {
    padding: 10px 14px;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 500;
    color: var(--fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.12em;
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
</style>
