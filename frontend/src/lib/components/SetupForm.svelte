<script lang="ts">
  import EquipmentAutocomplete from './EquipmentAutocomplete.svelte';
  import type { SetupDetail } from '$lib/api/SetupDetail';

  interface Props {
    initial: SetupDetail | null;
    cancelHref: string;
    submitLabel?: string;
  }
  let { initial, cancelHref, submitLabel = 'Save' }: Props = $props();

  function pickItem(role: string): string {
    if (!initial) return '';
    const it = initial.items.find((x) => x.role === role);
    return it ? it.item.display_name : '';
  }
  function pickFilters(): string[] {
    return (initial?.items ?? [])
      .filter((x) => x.role === 'filter')
      .map((x) => x.item.display_name);
  }

  let name = $state(initial?.name ?? '');
  let description = $state(initial?.description ?? '');
  let location = $state(initial?.location ?? '');
  let is_remote = $state(initial?.is_remote ?? false);
  let is_default = $state(initial?.is_default ?? false);
  let guiding = $state(initial?.guiding ?? '');

  let opticalText = $state(pickItem('optical_tube'));
  let focalText = $state(pickItem('focal_modifier'));
  let cameraText = $state(pickItem('main_camera'));
  let mountText = $state(pickItem('mount'));

  // Filter chips: array of display-name strings; resolved server-side on submit.
  let filterChips = $state<string[]>(pickFilters());
  let filterText = $state('');

  function addFilter(text: string) {
    const t = text.trim();
    if (!t) return;
    if (filterChips.includes(t)) return;
    filterChips = [...filterChips, t];
    filterText = '';
  }
  function removeFilter(t: string) {
    filterChips = filterChips.filter((f) => f !== t);
  }
</script>

<!-- No <form> wrapper — parent owns the form element -->
<div class="setup-form">
  <label class="field">
    <span class="t-label">Name</span>
    <input name="name" bind:value={name} required />
  </label>

  <label class="field">
    <span class="t-label">Description</span>
    <textarea name="description" bind:value={description} rows="2"></textarea>
  </label>

  <label class="field">
    <span class="t-label">Location</span>
    <input name="location" bind:value={location} placeholder="e.g., Backyard observatory" />
  </label>

  <div class="row">
    <label class="check">
      <input type="checkbox" name="is_remote" bind:checked={is_remote} />
      Remote
    </label>
    <label class="check" title="Auto-applied to new uploads">
      <input type="checkbox" name="is_default" bind:checked={is_default} />
      Default
    </label>
  </div>

  <fieldset class="equipment">
    <legend>Equipment</legend>

    <div class="field">
      <span class="t-label">Optical tube</span>
      <EquipmentAutocomplete
        name="optical_tube_text"
        kind="telescope"
        bind:value={opticalText}
        label={null}
      />
    </div>

    <div class="field">
      <span class="t-label">Focal modifier</span>
      <EquipmentAutocomplete
        name="focal_modifier_text"
        kind="focal_modifier"
        bind:value={focalText}
        label={null}
      />
    </div>

    <div class="field">
      <span class="t-label">Main camera</span>
      <EquipmentAutocomplete
        name="main_camera_text"
        kind="camera"
        bind:value={cameraText}
        label={null}
      />
    </div>

    <div class="field">
      <span class="t-label">Mount</span>
      <EquipmentAutocomplete name="mount_text" kind="mount" bind:value={mountText} label={null} />
    </div>

    <div class="field">
      <span class="t-label">Filters</span>
      <!-- Hidden inputs carry each chip value to the server -->
      {#each filterChips as f (f)}
        <input type="hidden" name="filter_text" value={f} />
      {/each}
      <ul class="chips">
        {#each filterChips as f (f)}
          <li class="chip">
            {f}
            <button
              type="button"
              class="chip-x"
              aria-label={`Remove ${f}`}
              onclick={() => removeFilter(f)}>×</button
            >
          </li>
        {/each}
      </ul>
      <EquipmentAutocomplete
        name="_filter_input"
        kind="filter"
        bind:value={filterText}
        onCommit={(c) => {
          if (c) addFilter(c.display_name);
        }}
        label={null}
      />
    </div>

    <label class="field">
      <span class="t-label">Guiding</span>
      <input
        name="guiding"
        bind:value={guiding}
        placeholder="e.g., ASI120MM Mini + 60mm guide scope"
      />
      <small class="hint">Free text. Not auto-completed.</small>
    </label>
  </fieldset>

  <div class="actions">
    <a href={cancelHref} class="btn ghost">Cancel</a>
    <button type="submit" class="btn primary">{submitLabel}</button>
  </div>
</div>

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
    text-decoration: none;
    display: inline-flex;
    align-items: center;
  }
  .btn.primary {
    background: var(--primary, #0a6);
    color: white;
    border: none;
  }
  .btn.ghost {
    background: transparent;
    border: 1px solid var(--border, #ccc);
    color: inherit;
  }
  .hint {
    color: var(--muted, #666);
    font-size: 0.85em;
  }
  .t-label {
    font-size: 0.85em;
    color: var(--muted, #666);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
</style>
