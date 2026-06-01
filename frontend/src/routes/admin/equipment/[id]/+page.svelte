<script lang="ts">
  import { untrack } from 'svelte';
  import { goto } from '$app/navigation';
  import Field from '$lib/components/equipment/Field.svelte';
  import BrandModelVariantPicker, {
    type BMV
  } from '$lib/components/equipment/BrandModelVariantPicker.svelte';
  import { FIELDS_BY_KIND, type SpecField } from '$lib/equipment/specs-fields';
  import { editEquipment, deleteEquipment } from '$lib/api/adminClient';
  import type { EquipmentSpecsPayload } from '$lib/api/EquipmentSpecsPayload';

  let { data } = $props();

  const STATUSES = ['approved', 'pending', 'rejected'];

  // Header fields — seeded once from the item; edits drive local state.
  let bmv = $state<BMV>(
    untrack(() => ({
      brand: data.item.brand,
      model: data.item.model,
      variant: data.item.variant ?? ''
    }))
  );
  let status = $state(untrack(() => data.item.status));

  // display_name is derived from brand/model/variant (the backend regenerates
  // it on save). Shown read-only as a live preview.
  let displayPreview = $derived.by(() => {
    const b = bmv.brand.trim();
    const m = bmv.model.trim();
    const v = bmv.variant.trim();
    const suffix = v ? ` ${v}` : '';
    return b ? `${b} ${m}${suffix}` : `${m}${suffix}`;
  });

  const fields: SpecField[] = $derived(
    (FIELDS_BY_KIND as Record<string, SpecField[]>)[data.item.kind] ?? []
  );

  // Lift specs out of the `{ kind, ...fields }` discriminated union so the
  // inputs can bind to scalar refs. Seeded once; user edits drive state.
  function seedSpecs(): Record<string, unknown> {
    if (!data.item.specs) return {};
    const { kind: _k, ...rest } = data.item.specs as unknown as Record<string, unknown>;
    return rest;
  }
  let specs = $state<Record<string, unknown>>(untrack(() => seedSpecs()));

  let saving = $state(false);
  let deleting = $state(false);
  let error = $state<string | null>(null);
  let saved = $state(false);

  let inUse = $derived(data.item.usage_count > 0 || Number(data.item.setup_count) > 0);

  function focalRatioPreview(s: Record<string, unknown>): string {
    const a = Number(s.aperture_mm);
    const f = Number(s.focal_length_mm);
    if (!a || !f) return '';
    return `f/${(f / a).toFixed(2)}`;
  }

  async function save() {
    saving = true;
    saved = false;
    error = null;
    try {
      // Build the per-kind specs payload: keep populated fields, drop empties
      // (replace-all semantics → empty clears). Skip DB-computed fields.
      const cleanedSpecs: Record<string, unknown> = { kind: data.item.kind };
      for (const f of fields) {
        if (f.type === 'computed') continue;
        const v = specs[f.name];
        if (v === '' || v === undefined || v === null) continue;
        cleanedSpecs[f.name] = f.type === 'number' ? Number(v) : v;
      }
      await editEquipment(fetch, data.item.id, {
        brand: bmv.brand,
        model: bmv.model,
        variant: bmv.variant,
        status,
        specs: cleanedSpecs as unknown as EquipmentSpecsPayload
      });
      saved = true;
      // Reload so the canonical name + any derived fields reflect the save.
      await goto(`/admin/equipment/${data.item.id}`, { invalidateAll: true, noScroll: true });
    } catch (e) {
      error = (e as Error).message;
    } finally {
      saving = false;
    }
  }

  async function remove() {
    if (!confirm(`Delete "${data.item.display_name}"? This cannot be undone.`)) return;
    deleting = true;
    error = null;
    try {
      await deleteEquipment(fetch, data.item.id);
      await goto('/admin/equipment');
    } catch (e) {
      error =
        (e as Error).message === 'in_use'
          ? 'Item is still used by photos or setups — cannot delete.'
          : (e as Error).message;
      deleting = false;
    }
  }
</script>

<svelte:head><title>{data.item.display_name} · Admin · Astrophoto</title></svelte:head>

<a class="back" href="/admin/equipment">← All equipment</a>

<header class="head">
  <h1>{data.item.display_name}</h1>
  <div class="meta">
    <span class="chip">{data.item.kind}</span>
    <span>·</span>
    <span>{data.item.usage_count} uses</span>
    <span>·</span>
    <span>{Number(data.item.setup_count)} setups</span>
    <span>·</span>
    <span>added by {data.item.submitted_by_handle ? `@${data.item.submitted_by_handle}` : '—'}</span
    >
  </div>
</header>

{#if error}<p class="err">{error}</p>{/if}

<form
  onsubmit={(e) => {
    e.preventDefault();
    void save();
  }}
>
  <fieldset class="section">
    <legend>Identity</legend>
    <p class="hint hint--top">
      Brand / model / variant are assisted from the existing catalog — pick a suggested value to
      avoid creating duplicates. The display name is regenerated from them on save.
    </p>
    <BrandModelVariantPicker kind={data.item.kind} value={bmv} onChange={(next) => (bmv = next)} />
    <div class="grid grid--secondary">
      <Field label="Display name" hint="Generated from brand / model / variant.">
        <input class="input" value={displayPreview} disabled />
      </Field>
      <Field label="Kind" hint="Recategorising is not supported here.">
        <input class="input" value={data.item.kind} disabled />
      </Field>
      <Field label="Status">
        <select class="select" bind:value={status}>
          {#each STATUSES as s}
            <option value={s}>{s}</option>
          {/each}
        </select>
      </Field>
    </div>
  </fieldset>

  <fieldset class="section">
    <legend>Specs · {data.item.kind}</legend>
    {#if fields.length === 0}
      <p class="hint">No structured specs for this kind.</p>
    {:else}
      <div class="grid">
        {#each fields as field (field.name)}
          {#if field.type === 'enum'}
            <Field label={field.label} hint={field.helpText ?? ''}>
              <select class="select" bind:value={specs[field.name]}>
                <option value="">—</option>
                {#each field.options as opt (opt.value)}
                  <option value={opt.value}>{opt.label}</option>
                {/each}
              </select>
            </Field>
          {:else if field.type === 'number'}
            <Field label={field.label} mono hint={field.helpText ?? ''}>
              <input
                class="input input-mono"
                type="number"
                min={field.min}
                max={field.max}
                step={field.step}
                bind:value={specs[field.name]}
              />
            </Field>
          {:else if field.type === 'text'}
            <Field label={field.label} mono hint={field.helpText ?? ''}>
              <input class="input input-mono" bind:value={specs[field.name]} />
            </Field>
          {:else if field.type === 'bool'}
            {@const checked = specs[field.name] === true}
            <Field label={field.label}>
              <label class="bool">
                <input
                  type="checkbox"
                  {checked}
                  onchange={(e) =>
                    (specs[field.name] = (e.currentTarget as HTMLInputElement).checked)}
                />
                {field.label}
              </label>
            </Field>
          {:else if field.type === 'computed'}
            <Field label={field.label} mono hint={field.helpText ?? ''}>
              <input
                class="input input-mono"
                readonly
                value={data.item.kind === 'telescope' ? focalRatioPreview(specs) : ''}
              />
            </Field>
          {/if}
        {/each}
      </div>
      <p class="hint">
        Specs are <strong>shared</strong> + replace-all — clearing a field clears it for every photo and
        setup using this item.
      </p>
    {/if}
  </fieldset>

  <div class="actions">
    <button type="submit" class="btn primary" disabled={saving || deleting}>
      {saving ? 'Saving…' : 'Save changes'}
    </button>
    {#if saved && !saving}<span class="ok">Saved ✓</span>{/if}
    <span class="spacer"></span>
    <button
      type="button"
      class="btn danger"
      disabled={saving || deleting || inUse}
      title={inUse ? 'In use — cannot delete' : 'Delete this item'}
      onclick={() => void remove()}
    >
      {deleting ? 'Deleting…' : 'Delete item'}
    </button>
  </div>
  {#if inUse}
    <p class="hint">Delete is disabled while photos or setups reference this item.</p>
  {/if}
</form>

<style>
  .back {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-muted);
    text-decoration: none;
  }
  .head {
    margin: 8px 0 20px;
  }
  h1 {
    font-family: var(--font-display, serif);
    font-weight: 400;
    margin: 4px 0 6px;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-muted);
  }
  .chip {
    color: var(--accent);
    text-transform: uppercase;
  }
  .section {
    border: 1px solid var(--border-subtle);
    padding: 16px;
    margin: 0 0 16px;
  }
  legend {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-muted);
    padding: 0 6px;
    text-transform: uppercase;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
  }
  .grid--secondary {
    margin-top: 16px;
  }
  .hint--top {
    margin: 0 0 14px;
  }
  @media (max-width: 720px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }
  .input,
  .select {
    width: 100%;
    background: var(--bg-canvas);
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 8px 10px;
    font-size: 14px;
  }
  .input-mono {
    font-family: var(--font-mono);
  }
  .input:disabled {
    opacity: 0.6;
  }
  .bool {
    display: inline-flex;
    gap: 8px;
    align-items: center;
    cursor: pointer;
    font-size: 13px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .spacer {
    flex: 1;
  }
  .btn {
    padding: 10px 16px;
    font-family: var(--font-mono);
    font-size: 12px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    cursor: pointer;
    border: 1px solid var(--border-subtle);
    background: var(--bg-canvas);
    color: var(--fg-primary);
  }
  .btn.primary {
    background: var(--accent);
    color: var(--accent-ink);
    border-color: var(--accent);
  }
  .btn.danger {
    border-color: var(--danger, #c33);
    color: var(--danger, #c33);
  }
  .btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .ok {
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .err {
    color: var(--danger, #c33);
    font-family: var(--font-mono);
    font-size: 12px;
    margin: 0 0 12px;
  }
  .hint {
    color: var(--fg-muted);
    font-family: var(--font-mono);
    font-size: 11px;
    margin: 12px 0 0;
  }
</style>
