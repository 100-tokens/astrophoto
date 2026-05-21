<script lang="ts">
  import { goto } from '$app/navigation';
  import { untrack } from 'svelte';
  import EquipmentAutocomplete from './EquipmentAutocomplete.svelte';
  import Field from '$lib/components/equipment/Field.svelte';
  import RoleRow from '$lib/components/equipment/RoleRow.svelte';
  import SpecsPanel from '$lib/components/equipment/SpecsPanel.svelte';
  import FilterChipInput from '$lib/components/equipment/FilterChipInput.svelte';
  // Catalog v2 (Phase 2): the in-line per-field grid was replaced with
  // the shared *SpecsForm components so create/edit shares one source
  // of truth with the FilterChipInput and EquipmentAutocomplete popovers.
  import TelescopeSpecsForm from '$lib/components/equipment/specs/TelescopeSpecsForm.svelte';
  import CameraSpecsForm from '$lib/components/equipment/specs/CameraSpecsForm.svelte';
  import MountSpecsForm from '$lib/components/equipment/specs/MountSpecsForm.svelte';
  import FocalModifierSpecsForm from '$lib/components/equipment/specs/FocalModifierSpecsForm.svelte';
  import '$lib/components/equipment/equipment-create-form.css';
  import type { SetupDetail } from '$lib/api/SetupDetail';
  import type { PhotoFilterChip } from '$lib/api/PhotoFilterChip';
  import type { EquipmentSpecsPayload } from '$lib/api/EquipmentSpecsPayload';
  import type { TelescopeSpecs } from '$lib/api/TelescopeSpecs';
  import type { CameraSpecs } from '$lib/api/CameraSpecs';
  import type { MountSpecs } from '$lib/api/MountSpecs';
  import type { FocalModifierSpecs } from '$lib/api/FocalModifierSpecs';

  // ── Item-level detail fetched server-side and passed in for edit mode ─────
  export type ItemPrefill = {
    id: string;
    display_name: string;
    specs: EquipmentSpecsPayload | null;
  };

  interface Props {
    /** null → create mode; populated → edit mode */
    initial: SetupDetail | null;
    /** Per-role prefill (id + specs), keyed by role. Only passed on edit. */
    prefill?: {
      optical_tube?: ItemPrefill;
      main_camera?: ItemPrefill;
      mount?: ItemPrefill;
      focal_modifier?: ItemPrefill;
      filters?: PhotoFilterChip[];
    };
    /** ID of the setup being edited (undefined on create) */
    setupId?: string;
  }

  let { initial, prefill = {}, setupId }: Props = $props();

  // ── Setup-level fields ────────────────────────────────────────────────────
  // untrack: these are one-time init captures — we don't want reactivity on prop changes.
  let setupName = $state(untrack(() => initial?.name ?? ''));
  let setupDescription = $state(untrack(() => initial?.description ?? null));
  let setupLocation = $state(untrack(() => initial?.location ?? null));
  let setupIsRemote = $state(untrack(() => initial?.is_remote ?? false));
  let setupIsDefault = $state(untrack(() => initial?.is_default ?? false));
  let setupGuiding = $state(untrack(() => initial?.guiding ?? null));

  // ── Per-role state ────────────────────────────────────────────────────────
  type RoleState = {
    itemId: string | null;
    name: string;
    specs: Record<string, unknown>;
    open: boolean;
    /** mode is 'edit' when item already exists in catalog, 'create' when new */
    mode: 'create' | 'edit';
  };

  function makeRole(prefillItem?: ItemPrefill): RoleState {
    if (prefillItem) {
      const specs: Record<string, unknown> = {};
      if (prefillItem.specs) {
        const payload = prefillItem.specs;
        // Strip the `kind` discriminator — rest are spec fields
        for (const [k, v] of Object.entries(payload)) {
          if (k !== 'kind') specs[k] = v;
        }
      }
      return {
        itemId: prefillItem.id,
        name: prefillItem.display_name,
        specs,
        open: false,
        mode: 'edit'
      };
    }
    return { itemId: null, name: '', specs: {}, open: false, mode: 'create' };
  }

  let telescope = $state<RoleState>(untrack(() => makeRole(prefill.optical_tube)));
  let camera = $state<RoleState>(untrack(() => makeRole(prefill.main_camera)));
  let mount = $state<RoleState>(untrack(() => makeRole(prefill.mount)));
  let focalMod = $state<RoleState>(untrack(() => makeRole(prefill.focal_modifier)));
  let filters = $state<PhotoFilterChip[]>(untrack(() => prefill.filters ?? []));

  // ── Derived ───────────────────────────────────────────────────────────────
  const focalRatio = $derived.by(() => {
    const apert = Number(telescope.specs['aperture_mm']);
    const focal = Number(telescope.specs['focal_length_mm']);
    if (apert > 0 && focal > 0) return (focal / apert).toFixed(2);
    return '';
  });

  const roleCount = $derived(
    [telescope, camera, mount, focalMod].filter((r) => r.itemId !== null).length
  );
  const filterCount = $derived(filters.length);
  const readyMeta = $derived(
    `${roleCount} ROLE${roleCount !== 1 ? 'S' : ''} · ${filterCount} FILTER${filterCount !== 1 ? 'S' : ''}`
  );

  // ── Error state ───────────────────────────────────────────────────────────
  let error = $state<string | null>(null);
  let saving = $state(false);

  // ── Apply behavior ─ persisted as setup.default_apply_mode ───────────────
  // Backed by equipment_setups.default_apply_mode (migration 0019). The
  // apply-setup endpoint still requires `mode` in its request body; this
  // field is just the user's per-setup default that the verify form reads
  // to pre-select the right radio.
  let applyBehavior = $state<'overwrite' | 'fill_empty'>(
    untrack(() => (initial?.default_apply_mode === 'fill_empty' ? 'fill_empty' : 'overwrite'))
  );

  // ── Helpers ───────────────────────────────────────────────────────────────
  function onRoleCommit(role: RoleState, committed: { id: string; display_name: string } | null) {
    role.itemId = committed?.id ?? null;
    role.name = committed?.display_name ?? '';
    if (committed) {
      // Switched to a (possibly existing) item — assume edit mode
      role.mode = 'edit';
    }
  }

  async function saveSpecs(role: RoleState, kind: string): Promise<string | null> {
    const name = role.name.trim();
    if (!name) return null;

    // Build specs payload — omit computed fields
    const specsFields: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(role.specs)) {
      if (k === 'focal_ratio_f') continue; // DB-generated
      if (v !== '' && v !== null && v !== undefined) {
        specsFields[k] = v;
      }
    }
    const hasSpecs = Object.keys(specsFields).length > 0;

    if (role.itemId) {
      // PATCH existing item specs
      if (hasSpecs) {
        const r = await fetch(`/api/equipment/items/${role.itemId}`, {
          method: 'PATCH',
          credentials: 'include',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({ specs: { kind, ...specsFields } })
        });
        if (!r.ok) throw new Error(`Failed to update ${kind} specs`);
      }
      return role.itemId;
    } else {
      // POST new item
      const r = await fetch('/api/equipment/items', {
        method: 'POST',
        credentials: 'include',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          kind,
          display_name: name,
          specs: hasSpecs ? { kind, ...specsFields } : null
        })
      });
      if (!r.ok) throw new Error(`Failed to create ${kind} item`);
      const row: { id: string } = await r.json();
      role.itemId = row.id;
      return row.id;
    }
  }

  async function saveSpecsForRole(role: RoleState, kind: string) {
    try {
      await saveSpecs(role, kind);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Save failed';
    }
  }

  async function saveSetup() {
    const name = setupName.trim();
    if (!name) {
      error = 'Setup name is required';
      return;
    }
    saving = true;
    error = null;
    try {
      // Resolve each role's item
      const roleMap: Array<[RoleState, string, string]> = [
        [telescope, 'optical_tube', 'telescope'],
        [camera, 'main_camera', 'camera'],
        [mount, 'mount', 'mount'],
        [focalMod, 'focal_modifier', 'focal_modifier']
      ];

      const items: { role: string; item_id: string }[] = [];

      for (const [role, roleName, kind] of roleMap) {
        if (!role.name.trim()) continue;
        const id = await saveSpecs(role, kind);
        if (id) items.push({ role: roleName, item_id: id });
      }

      for (const f of filters) {
        items.push({ role: 'filter', item_id: f.id });
      }

      const body = {
        name,
        description: setupDescription,
        location: setupLocation,
        is_remote: setupIsRemote,
        is_default: setupIsDefault,
        guiding: setupGuiding,
        default_apply_mode: applyBehavior,
        items
      };

      const url = setupId ? `/api/equipment/setups/${setupId}` : '/api/equipment/setups';
      const method = setupId ? 'PATCH' : 'POST';

      const r = await fetch(url, {
        method,
        credentials: 'include',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(body)
      });

      if (!r.ok) {
        let msg = `Backend error (${r.status})`;
        try {
          const b: unknown = await r.json();
          if (typeof b === 'object' && b !== null && 'error' in b && typeof b.error === 'string') {
            msg = b.error;
          }
        } catch {
          // ignore
        }
        error = msg;
        return;
      }

      await goto('/settings/equipment');
    } catch (e) {
      error = e instanceof Error ? e.message : 'Unexpected error';
    } finally {
      saving = false;
    }
  }

  // ── Typed views over role.specs ───────────────────────────────────────────
  //
  // The shared *SpecsForm components are strictly typed; role.specs is
  // a Record<string, unknown> at the SetupForm level (so all four roles
  // share one shape). These $derived getters pull a typed view out of
  // role.specs for the form; the setters spread the typed updates back
  // into role.specs. Missing fields default to null which is what the
  // backend expects.
  function pickTelescope(s: Record<string, unknown>): TelescopeSpecs {
    return {
      design: (s.design as TelescopeSpecs['design']) ?? null,
      aperture_mm: (s.aperture_mm as number | null) ?? null,
      focal_length_mm: (s.focal_length_mm as number | null) ?? null,
      focal_ratio_f: (s.focal_ratio_f as number | null) ?? null,
      self_weight_kg: (s.self_weight_kg as number | null) ?? null,
      optical_length_mm: (s.optical_length_mm as number | null) ?? null,
      backfocus_mm: (s.backfocus_mm as number | null) ?? null
    };
  }
  function pickCamera(s: Record<string, unknown>): CameraSpecs {
    return {
      sensor_type: (s.sensor_type as CameraSpecs['sensor_type']) ?? null,
      color_type: (s.color_type as CameraSpecs['color_type']) ?? null,
      cooled: (s.cooled as boolean | null) ?? null,
      sensor_model: (s.sensor_model as string | null) ?? null,
      pixel_size_um: (s.pixel_size_um as number | null) ?? null,
      sensor_width_px: (s.sensor_width_px as number | null) ?? null,
      sensor_height_px: (s.sensor_height_px as number | null) ?? null,
      self_weight_g: (s.self_weight_g as number | null) ?? null,
      full_well_capacity_e: (s.full_well_capacity_e as number | null) ?? null,
      read_noise_e: (s.read_noise_e as number | null) ?? null,
      mount_thread: (s.mount_thread as string | null) ?? null,
      backfocus_mm: (s.backfocus_mm as number | null) ?? null
    };
  }
  function pickMount(s: Record<string, unknown>): MountSpecs {
    return {
      mount_type: (s.mount_type as MountSpecs['mount_type']) ?? null,
      payload_kg: (s.payload_kg as number | null) ?? null,
      goto: (s.goto as boolean | null) ?? null,
      self_weight_kg: (s.self_weight_kg as number | null) ?? null,
      periodic_error_arcsec: (s.periodic_error_arcsec as number | null) ?? null,
      tripod_included: (s.tripod_included as boolean | null) ?? null,
      control_protocol: (s.control_protocol as string | null) ?? null
    };
  }
  function pickFocalMod(s: Record<string, unknown>): FocalModifierSpecs {
    return {
      modifier_type: (s.modifier_type as FocalModifierSpecs['modifier_type']) ?? null,
      factor: (s.factor as number | null) ?? null,
      self_weight_g: (s.self_weight_g as number | null) ?? null,
      backfocus_mm: (s.backfocus_mm as number | null) ?? null,
      image_circle_mm: (s.image_circle_mm as number | null) ?? null
    };
  }

  // Setter: merge the typed update back into role.specs. The form may
  // emit null for cleared fields, which is what saveSpecs already
  // filters out before POST/PATCH.
  function setSpecs(role: RoleState, next: Record<string, unknown>) {
    role.specs = { ...role.specs, ...next };
  }
</script>

<div class="setup-builder">
  {#if error}
    <p class="form-error">{error}</p>
  {/if}

  <div class="builder-body">
    <!-- ── Left column ─────────────────────────────────────────────────── -->
    <div class="builder-main">
      <!-- Setup name + location row -->
      <div class="header-fields">
        <Field label="Setup name" hint="A short label — visible on every frame using this setup.">
          <input
            class="input"
            value={setupName}
            oninput={(e) => (setupName = (e.target as HTMLInputElement).value)}
            placeholder="e.g. Backyard SHO @ Bortle 4"
          />
        </Field>
        <Field label="Default site" hint="Optional · prefills location on apply.">
          <input
            class="input"
            value={setupLocation ?? ''}
            oninput={(e) => {
              const v = (e.target as HTMLInputElement).value.trim();
              setupLocation = v || null;
            }}
            placeholder="e.g. Backyard observatory"
          />
        </Field>
      </div>

      <div class="t-label roles-label">ROLES</div>

      <!-- Telescope -->
      <RoleRow
        kind="TELESCOPE"
        value={telescope.name}
        expanded={telescope.open}
        onToggle={() => (telescope.open = !telescope.open)}
      >
        {#snippet input()}
          <EquipmentAutocomplete
            name="telescope_name"
            kind="telescope"
            bind:value={telescope.name}
            label={null}
            onCommit={(c) => onRoleCommit(telescope, c)}
          />
        {/snippet}
        {#snippet children()}
          <SpecsPanel
            mode={telescope.mode}
            footerNote={telescope.mode === 'edit'
              ? 'Edits write to the shared catalog — affects all users with this telescope.'
              : ''}
            onSave={() => saveSpecsForRole(telescope, 'telescope')}
            onDiscard={() => (telescope.open = false)}
          >
            <TelescopeSpecsForm
              value={pickTelescope(telescope.specs)}
              onChange={(next) => setSpecs(telescope, next)}
            />
            {#if focalRatio}
              <div class="callout-db">
                <span class="t-label">DB-GENERATED</span>
                <span class="callout-db-body">
                  <code>focal_ratio_f</code> · <strong>{focalRatio}</strong> · STORED column,
                  recomputed from <code>focal_length_mm / aperture_mm</code> on save.
                </span>
              </div>
            {/if}
          </SpecsPanel>
        {/snippet}
      </RoleRow>

      <!-- Camera -->
      <RoleRow
        kind="CAMERA"
        value={camera.name}
        expanded={camera.open}
        onToggle={() => (camera.open = !camera.open)}
      >
        {#snippet input()}
          <EquipmentAutocomplete
            name="camera_name"
            kind="camera"
            bind:value={camera.name}
            label={null}
            onCommit={(c) => onRoleCommit(camera, c)}
          />
        {/snippet}
        {#snippet children()}
          <SpecsPanel
            mode={camera.mode}
            footerNote={camera.mode === 'edit'
              ? 'Edits write to the shared catalog — affects all users with this camera.'
              : ''}
            onSave={() => saveSpecsForRole(camera, 'camera')}
            onDiscard={() => (camera.open = false)}
          >
            <CameraSpecsForm
              value={pickCamera(camera.specs)}
              onChange={(next) => setSpecs(camera, next)}
            />
          </SpecsPanel>
        {/snippet}
      </RoleRow>

      <!-- Mount -->
      <RoleRow
        kind="MOUNT"
        value={mount.name}
        expanded={mount.open}
        onToggle={() => (mount.open = !mount.open)}
      >
        {#snippet input()}
          <EquipmentAutocomplete
            name="mount_name"
            kind="mount"
            bind:value={mount.name}
            label={null}
            onCommit={(c) => onRoleCommit(mount, c)}
          />
        {/snippet}
        {#snippet children()}
          <SpecsPanel
            mode={mount.mode}
            footerNote={mount.mode === 'edit'
              ? 'Edits write to the shared catalog — affects all users with this mount.'
              : ''}
            onSave={() => saveSpecsForRole(mount, 'mount')}
            onDiscard={() => (mount.open = false)}
          >
            <MountSpecsForm
              value={pickMount(mount.specs)}
              onChange={(next) => setSpecs(mount, next)}
            />
          </SpecsPanel>
        {/snippet}
      </RoleRow>

      <!-- Focal modifier -->
      <RoleRow
        kind="FOCAL MODIFIER"
        value={focalMod.name}
        expanded={focalMod.open}
        onToggle={() => (focalMod.open = !focalMod.open)}
      >
        {#snippet input()}
          <EquipmentAutocomplete
            name="focal_modifier_name"
            kind="focal_modifier"
            bind:value={focalMod.name}
            label={null}
            onCommit={(c) => onRoleCommit(focalMod, c)}
          />
        {/snippet}
        {#snippet children()}
          <SpecsPanel
            mode={focalMod.mode}
            footerNote={focalMod.mode === 'edit'
              ? 'Edits write to the shared catalog — affects all users with this item.'
              : ''}
            onSave={() => saveSpecsForRole(focalMod, 'focal_modifier')}
            onDiscard={() => (focalMod.open = false)}
          >
            <FocalModifierSpecsForm
              value={pickFocalMod(focalMod.specs)}
              onChange={(next) => setSpecs(focalMod, next)}
            />
          </SpecsPanel>
        {/snippet}
      </RoleRow>

      <!-- Filters row (multi-select chip input) -->
      <div class="role-row filters-row">
        <div class="filters-head">
          <div>
            <span class="t-label">FILTERS</span>
            <div class="t-meta filters-meta">MULTI · ORDERED</div>
          </div>
          <div class="filters-body">
            <FilterChipInput value={filters} onChange={(next) => (filters = next)} />
            <div class="t-meta filters-hint">
              The filter list drives <code>photo_filters</code> when this setup is applied. Order here
              is the canonical order shown on photos.
            </div>
          </div>
        </div>
      </div>

      <!-- Save row -->
      <div class="save-row">
        <span class="t-meta">{readyMeta}</span>
        <span class="save-row-spacer"></span>
        <a href="/settings/equipment" class="btn btn-ghost btn-lg">Cancel</a>
        <button type="button" class="btn btn-primary btn-lg" onclick={saveSetup} disabled={saving}>
          {saving ? 'Saving…' : setupId ? 'Save changes' : 'Save setup'}
        </button>
      </div>
    </div>

    <!-- ── Right rail ──────────────────────────────────────────────────── -->
    <aside class="builder-aside">
      <div class="callout">
        <div class="t-label callout-label">SHARED CATALOG</div>
        <p class="callout-body">
          Equipment items are <strong>shared</strong> across astrophoto.pics. Editing specs on a role
          updates the catalog row for everyone using that item.
        </p>
        <p class="callout-meta">
          Phase 1: any signed-in user can edit. Moderation queue ships in phase 2.
        </p>
      </div>

      <div class="callout">
        <div class="t-label callout-label">APPLY BEHAVIOR</div>
        <label class="apply-option">
          <input
            type="radio"
            name="apply_behavior"
            value="overwrite"
            checked={applyBehavior === 'overwrite'}
            onchange={() => (applyBehavior = 'overwrite')}
          />
          <span>
            <strong class="apply-option-title">Overwrite</strong>
            <span class="apply-option-desc"
              >Replace existing equipment fields and filter junction on apply.</span
            >
          </span>
        </label>
        <label class="apply-option">
          <input
            type="radio"
            name="apply_behavior"
            value="fill_empty"
            checked={applyBehavior === 'fill_empty'}
            onchange={() => (applyBehavior = 'fill_empty')}
          />
          <span>
            <strong class="apply-option-title">Fill empty</strong>
            <span class="apply-option-desc">Only set fields that are currently blank.</span>
          </span>
        </label>
      </div>
    </aside>
  </div>
</div>

<style>
  .setup-builder {
    width: 100%;
  }

  .form-error {
    color: var(--danger);
    font-size: 13px;
    margin-bottom: 16px;
    padding: 10px 14px;
    border: 1px solid var(--danger);
    background: color-mix(in srgb, var(--danger) 8%, var(--bg-base));
  }

  .builder-body {
    display: grid;
    /* In the actual settings shell the SetupForm renders inside a
       ~592px main column (240px left nav + 64px*2 settings-shell
       padding + 64px*2 page-body padding eat the 1440px viewport).
       That's not enough to host the handoff's 1fr+340px split — the
       SpecsPanel inputs collide with the aside. Stack vertically
       (aside on top so the "SHARED CATALOG" + "APPLY BEHAVIOR"
       context reads before the form). The two columns return on
       very wide viewports where the math works. */
    grid-template-columns: 1fr;
    gap: 32px;
  }

  .builder-aside {
    order: -1;
  }

  .builder-main {
    min-width: 0;
  }

  @media (min-width: 1600px) {
    .builder-body {
      grid-template-columns: minmax(420px, 1fr) 340px;
      gap: 48px;
    }
    .builder-aside {
      order: 0;
    }
  }

  /* Header fields row */
  .header-fields {
    display: grid;
    grid-template-columns: 1fr 280px;
    gap: 24px;
    margin-bottom: 32px;
  }

  .roles-label {
    margin-bottom: 8px;
  }

  /* DB-GENERATED callout under focal_ratio_f in telescope specs */
  .callout-db {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 16px;
    background: var(--bg-accent-tint, rgba(232, 164, 58, 0.07));
    border-left: 2px solid var(--accent);
    font-size: 12px;
    color: var(--fg-secondary);
    line-height: 1.5;
    grid-column: span 2;
  }

  .callout-db .t-label {
    color: var(--accent);
  }

  .callout-db code {
    font-family: var(--font-mono);
    color: var(--fg-primary);
  }

  .callout-db strong {
    color: var(--accent);
    font-family: var(--font-mono);
    font-weight: 600;
  }

  /* Filters role row */
  .filters-row {
    border-top: 1px solid var(--border-subtle);
    padding: 20px 0;
  }

  .filters-head {
    display: grid;
    grid-template-columns: 140px 1fr;
    gap: 16px;
    align-items: flex-start;
  }

  .filters-meta {
    margin-top: 4px;
  }

  .filters-body {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .filters-hint {
    color: var(--fg-muted);
    font-size: 11px;
  }

  .filters-hint code {
    font-family: var(--font-mono);
  }

  /* Save row */
  .save-row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 40px;
    padding-top: 24px;
    border-top: 1px solid var(--border-subtle);
  }

  .save-row-spacer {
    flex: 1;
  }

  /* Right rail */
  .builder-aside {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .callout {
    padding: 20px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-raised);
  }

  .callout-label {
    margin-bottom: 12px;
  }

  .callout-body {
    margin: 0;
    font-size: 13px;
    color: var(--fg-secondary);
    line-height: 1.6;
  }

  .callout-body strong {
    color: var(--fg-primary);
  }

  .callout-meta {
    margin: 12px 0 0;
    font-size: 12px;
    color: var(--fg-muted);
  }

  .apply-option {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    margin-bottom: 16px;
    cursor: pointer;
  }

  .apply-option:last-child {
    margin-bottom: 0;
  }

  .apply-option input[type='radio'] {
    accent-color: var(--accent);
    margin-top: 3px;
    flex-shrink: 0;
  }

  .apply-option-title {
    display: block;
    font-size: 13px;
    color: var(--fg-primary);
  }

  .apply-option-desc {
    color: var(--fg-muted);
    font-size: 12px;
  }

  /* header-fields stays side-by-side until the viewport really
     can't host two columns. */
  @media (max-width: 900px) {
    .header-fields {
      grid-template-columns: 1fr;
    }
  }
</style>
