<script lang="ts">
  import EquipmentAutocomplete from './EquipmentAutocomplete.svelte';
  import type { SetupDetail } from '$lib/api/SetupDetail';
  import type { SetupInput } from '$lib/api/SetupInput';

  type Committed = { id: string; display_name: string };

  interface Props {
    initial: SetupDetail | null;
    submitLabel?: string;
    onsubmit: (input: SetupInput) => void;
    oncancel?: () => void;
  }
  let { initial, submitLabel = 'Save', onsubmit, oncancel }: Props = $props();

  function pickItem(role: string): Committed | null {
    if (!initial) return null;
    const it = initial.items.find((x) => x.role === role);
    return it ? { id: it.item.id, display_name: it.item.display_name } : null;
  }
  function pickFilters(): Committed[] {
    return (initial?.items ?? [])
      .filter((x) => x.role === 'filter')
      .map((x) => ({ id: x.item.id, display_name: x.item.display_name }));
  }

  let name = $state(initial?.name ?? '');
  let description = $state(initial?.description ?? '');
  let location = $state(initial?.location ?? '');
  let is_remote = $state(initial?.is_remote ?? false);
  let is_default = $state(initial?.is_default ?? false);
  let guiding = $state(initial?.guiding ?? '');

  let optical = $state<Committed | null>(pickItem('optical_tube'));
  let focal = $state<Committed | null>(pickItem('focal_modifier'));
  let camera = $state<Committed | null>(pickItem('main_camera'));
  let mount = $state<Committed | null>(pickItem('mount'));
  let filters = $state<Committed[]>(pickFilters());

  // Free-typed values, separate from the committed canonical id so the
  // input box reflects what the user typed even before the commit fires.
  let opticalText = $state(optical?.display_name ?? '');
  let focalText = $state(focal?.display_name ?? '');
  let cameraText = $state(camera?.display_name ?? '');
  let mountText = $state(mount?.display_name ?? '');
  let filterText = $state('');

  function addFilter(c: Committed | null) {
    if (!c) return;
    if (filters.some((f) => f.id === c.id)) return;
    filters = [...filters, c];
    filterText = '';
  }
  function removeFilter(id: string) {
    filters = filters.filter((f) => f.id !== id);
  }

  let error = $state<string | null>(null);

  function submit() {
    if (!name.trim()) {
      error = 'Name is required';
      return;
    }
    error = null;
    const items: SetupInput['items'] = [];
    if (optical) items.push({ role: 'optical_tube', item_id: optical.id });
    if (focal) items.push({ role: 'focal_modifier', item_id: focal.id });
    if (camera) items.push({ role: 'main_camera', item_id: camera.id });
    if (mount) items.push({ role: 'mount', item_id: mount.id });
    for (const f of filters) items.push({ role: 'filter', item_id: f.id });
    onsubmit({
      name: name.trim(),
      description: description.trim() || null,
      location: location.trim() || null,
      is_remote,
      is_default,
      guiding: guiding.trim() || null,
      items
    });
  }
</script>

<form
  onsubmit={(e) => {
    e.preventDefault();
    submit();
  }}
  class="setup-form"
>
  <label class="field">
    <span class="t-label">Name</span>
    <input bind:value={name} required />
  </label>

  <label class="field">
    <span class="t-label">Description</span>
    <textarea bind:value={description} rows="2"></textarea>
  </label>

  <label class="field">
    <span class="t-label">Location</span>
    <input bind:value={location} placeholder="e.g., Backyard observatory" />
  </label>

  <div class="row">
    <label class="check">
      <input type="checkbox" bind:checked={is_remote} />
      Remote
    </label>
    <label class="check" title="Auto-applied to new uploads">
      <input type="checkbox" bind:checked={is_default} />
      Default
    </label>
  </div>

  <fieldset class="equipment">
    <legend>Equipment</legend>

    <div class="field">
      <span class="t-label">Optical tube</span>
      <EquipmentAutocomplete
        name="optical_tube"
        kind="telescope"
        bind:value={opticalText}
        onCommit={(c) => (optical = c)}
      />
    </div>

    <div class="field">
      <span class="t-label">Focal modifier</span>
      <EquipmentAutocomplete
        name="focal_modifier"
        kind="focal_modifier"
        bind:value={focalText}
        onCommit={(c) => (focal = c)}
      />
    </div>

    <div class="field">
      <span class="t-label">Main camera</span>
      <EquipmentAutocomplete
        name="main_camera"
        kind="camera"
        bind:value={cameraText}
        onCommit={(c) => (camera = c)}
      />
    </div>

    <div class="field">
      <span class="t-label">Mount</span>
      <EquipmentAutocomplete
        name="mount"
        kind="mount"
        bind:value={mountText}
        onCommit={(c) => (mount = c)}
      />
    </div>

    <div class="field">
      <span class="t-label">Filters</span>
      <ul class="chips">
        {#each filters as f (f.id)}
          <li class="chip">
            {f.display_name}
            <button
              type="button"
              class="chip-x"
              aria-label={`Remove ${f.display_name}`}
              onclick={() => removeFilter(f.id)}>×</button
            >
          </li>
        {/each}
      </ul>
      <EquipmentAutocomplete
        name="filter"
        kind="filter"
        bind:value={filterText}
        onCommit={addFilter}
      />
    </div>

    <label class="field">
      <span class="t-label">Guiding</span>
      <input bind:value={guiding} placeholder="e.g., ASI120MM Mini + 60mm guide scope" />
      <small class="hint">Free text. Not auto-completed.</small>
    </label>
  </fieldset>

  {#if error}<p class="form-error">{error}</p>{/if}

  <div class="actions">
    {#if oncancel}
      <button type="button" class="btn ghost" onclick={() => oncancel?.()}>Cancel</button>
    {/if}
    <button type="submit" class="btn primary">{submitLabel}</button>
  </div>
</form>

<style>
  .setup-form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .row {
    display: flex;
    gap: 1.5rem;
  }
  .check {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }
  .equipment {
    border: 1px solid var(--border, #ccc);
    padding: 1rem;
    border-radius: 4px;
  }
  .chips {
    list-style: none;
    padding: 0;
    margin: 0 0 0.5rem 0;
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
  }
  .chip {
    background: var(--chip-bg, #eee);
    padding: 0.15rem 0.5rem;
    border-radius: 999px;
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
  }
  .chip-x {
    border: none;
    background: transparent;
    cursor: pointer;
    padding: 0 0.25rem;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
  }
  .btn {
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
  }
  .btn.primary {
    background: var(--primary, #0a6);
    color: white;
    border: none;
  }
  .btn.ghost {
    background: transparent;
    border: 1px solid var(--border, #ccc);
  }
  .hint {
    color: var(--muted, #666);
    font-size: 0.85em;
  }
  .form-error {
    color: var(--error, #c00);
  }
  .t-label {
    font-size: 0.85em;
    color: var(--muted, #666);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
</style>
