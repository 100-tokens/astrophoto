<script lang="ts">
  import { goto } from '$app/navigation';
  import { untrack } from 'svelte';
  import SpecsPanel from '$lib/components/equipment/SpecsPanel.svelte';
  import Field from '$lib/components/equipment/Field.svelte';
  import { FIELDS_BY_KIND, type SpecField } from '$lib/equipment/specs-fields';

  let { data } = $props();

  const fields: SpecField[] = $derived(
    (FIELDS_BY_KIND as Record<string, SpecField[]>)[data.item.kind] ?? []
  );

  // Initial specs values lifted out of the discriminated union so the
  // SpecsPanel inputs can bind to scalar refs. We never re-read `data`
  // beyond this seed — the user's edits drive the state.
  function seedSpecs(item: typeof data.item): Record<string, unknown> {
    if (!item.specs) return {};
    const { kind: _k, ...rest } = item.specs as unknown as Record<string, unknown>;
    return rest;
  }
  // The seed captures `data.item` at mount; the user's edits drive the
  // state from there. Wrapping in $derived would reset the user's edits
  // on every reactivity tick. `untrack` makes the one-shot read explicit
  // so svelte-check stops flagging it as a missed reactivity dependency.
  let specs = $state<Record<string, unknown>>(untrack(() => seedSpecs(data.item)));
  let saving = $state(false);
  let error = $state<string | null>(null);

  function focalRatioPreview(s: Record<string, unknown>): string {
    const a = Number(s.aperture_mm);
    const f = Number(s.focal_length_mm);
    if (!a || !f) return '';
    return `f/${(f / a).toFixed(2)}`;
  }

  async function save() {
    saving = true;
    error = null;
    try {
      // Drop computed fields and empty strings before submission.
      const cleaned: Record<string, unknown> = { kind: data.item.kind };
      for (const f of fields) {
        if (f.type === 'computed') continue;
        const v = specs[f.name];
        if (v === '' || v === undefined || v === null) continue;
        cleaned[f.name] = f.type === 'number' ? Number(v) : v;
      }

      const r = await fetch(`/api/equipment/items/${data.item.id}`, {
        method: 'PATCH',
        credentials: 'include',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ specs: cleaned })
      });
      if (!r.ok) {
        const body = await r.text();
        throw new Error(`backend ${r.status}: ${body.slice(0, 200)}`);
      }
      await goto(`/equip/${data.equipment.kind}/${data.equipment.slug}`);
    } catch (e) {
      error = (e as Error).message;
    } finally {
      saving = false;
    }
  }

  function discard() {
    void goto(`/equip/${data.equipment.kind}/${data.equipment.slug}`);
  }
</script>

<svelte:head>
  <title>Edit specs · {data.item.display_name} — Astrophoto</title>
</svelte:head>

<section class="edit-section">
  <div class="t-eyebrow">CATALOG · EDIT SPECS</div>
  <h1>{data.item.display_name}</h1>
  <p class="t-meta">
    Catalog items are <strong>shared</strong>. Saving here updates the specs row for every user
    referencing this item.
  </p>

  <SpecsPanel
    mode={data.item.specs === null ? 'create' : 'edit'}
    footerNote={data.item.specs === null
      ? 'Adding specs to a previously bare catalog item.'
      : 'Edits replace the existing specs row — leave a field empty to clear it.'}
    onSave={save}
    onDiscard={discard}
  >
    <div class="specs-grid">
      {#each fields as field (field.name)}
        {#if field.type === 'enum'}
          <Field label={field.label}>
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
  </SpecsPanel>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if saving}
    <p class="t-meta">Saving…</p>
  {/if}
</section>

<style>
  .edit-section {
    padding: 48px 64px;
    max-width: 880px;
    margin: 0 auto;
  }
  .specs-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 20px;
  }
  @media (max-width: 720px) {
    .specs-grid {
      grid-template-columns: 1fr;
    }
  }
  .error {
    color: var(--danger);
    font-family: var(--font-mono);
    font-size: 13px;
    margin-top: 16px;
  }
  .bool {
    display: inline-flex;
    gap: 8px;
    align-items: center;
    cursor: pointer;
  }
</style>
