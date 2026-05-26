<script lang="ts">
  import EquipmentAutocomplete from '$lib/components/EquipmentAutocomplete.svelte';
  import FilterChipInput from '$lib/components/equipment/FilterChipInput.svelte';
  import SetupPicker from '$lib/components/SetupPicker.svelte';
  import FieldShell from './FieldShell.svelte';
  import type { PhotoFilterChip } from '$lib/api/PhotoFilterChip';
  import type { SetupSummary } from '$lib/api/SetupSummary';

  // EquipmentSection — setup combobox + 2-col equipment grid + filter row.
  // Top-of-section setup picker stays SetupPicker (existing combobox);
  // when a setup is attached, we render the tinted "applied" pill above
  // the picker so the user can see what's active and detach. The picker
  // itself still drives onapply/ondetach to keep the conflict-detection
  // flow intact.

  interface SetupAppliedSpec {
    name: string;
    summary: string;
    setupIdShort: string;
  }

  interface Props {
    setups: SetupSummary[];
    currentSetupId: string | null;
    /** Currently-applied setup summary, when one is attached. */
    appliedSpec?: SetupAppliedSpec | null;
    camera: string;
    scope: string;
    focal_modifier: string;
    mount: string;
    guiding: string;
    filtersString: string;
    filterChips: PhotoFilterChip[];
    orphans: string[];
    startFilterOpen?: boolean;
    fromExif?: Set<string>;
    fromSetup?: Set<string>;
    disabled?: boolean;
    onApply: (req: { setup_id: string; mode: 'fill_empty' | 'overwrite' }) => void;
    onDetach: () => void;
    onChipsChange: (next: PhotoFilterChip[]) => void;
  }

  let {
    setups,
    currentSetupId,
    appliedSpec = null,
    camera = $bindable(''),
    scope = $bindable(''),
    focal_modifier = $bindable(''),
    mount = $bindable(''),
    guiding = $bindable(''),
    filtersString,
    filterChips,
    orphans = [],
    startFilterOpen = false,
    fromExif = new Set<string>(),
    fromSetup = new Set<string>(),
    disabled = false,
    onApply,
    onDetach,
    onChipsChange
  }: Props = $props();

  function sourceFor(k: string): 'exif' | 'setup' | null {
    if (fromSetup.has(k)) return 'setup';
    if (fromExif.has(k)) return 'exif';
    return null;
  }
</script>

<section class="equip">
  <div class="t-label equip-head">EQUIPMENT</div>

  <div class="equip-setup">
    {#if appliedSpec && currentSetupId}
      <!-- Applied setup: show the tinted summary + Detach. The combobox is
           hidden because keeping both on screen reads as duplicate state.
           Detach drops back to the combobox for picking a different setup. -->
      <div class="equip-applied">
        <span class="equip-applied-square" aria-hidden="true"></span>
        <span class="equip-applied-name">{appliedSpec.name}</span>
        {#if appliedSpec.summary}
          <span class="t-meta equip-applied-summary">{appliedSpec.summary}</span>
        {/if}
        <span class="equip-applied-spacer"></span>
        <span class="t-meta equip-applied-id">setup_id · {appliedSpec.setupIdShort}</span>
        <button type="button" class="btn btn-ghost btn-sm" onclick={onDetach} {disabled}
          >Detach</button
        >
      </div>
    {:else}
      <SetupPicker
        {setups}
        {currentSetupId}
        current={{ scope, focal_modifier, camera, mount, filters: filtersString, guiding }}
        onapply={onApply}
        ondetach={onDetach}
      />
    {/if}
  </div>

  <div class="equip-grid">
    <div class="equip-field">
      <FieldShell label="CAMERA" source={sourceFor('camera')}>
        <EquipmentAutocomplete name="camera" kind="camera" bind:value={camera} label={null} />
      </FieldShell>
    </div>
    <div class="equip-field">
      <FieldShell label="TELESCOPE" source={sourceFor('scope')}>
        <EquipmentAutocomplete name="scope" kind="telescope" bind:value={scope} label={null} />
      </FieldShell>
    </div>
    <div class="equip-field">
      <FieldShell label="FOCAL MODIFIER" source={sourceFor('focal_modifier')}>
        <EquipmentAutocomplete
          name="focal_modifier"
          kind="focal_modifier"
          bind:value={focal_modifier}
          label={null}
        />
      </FieldShell>
    </div>
    <div class="equip-field">
      <FieldShell label="MOUNT" source={sourceFor('mount')}>
        <EquipmentAutocomplete name="mount" kind="mount" bind:value={mount} label={null} />
      </FieldShell>
    </div>
    <div class="equip-field equip-field--full">
      <FieldShell label="GUIDING" full source={sourceFor('guiding')}>
        <EquipmentAutocomplete name="guiding" kind="guiding" bind:value={guiding} label={null} />
      </FieldShell>
    </div>
  </div>

  <div class="equip-filters">
    <div class="equip-filters-head">
      <div class="t-label">FILTERS · STRUCTURED</div>
      <span class="t-meta equip-filters-meta">
        {filterChips.length} CHOSEN · DRAG TO REORDER
      </span>
    </div>
    <FilterChipInput
      value={filterChips}
      {orphans}
      startOpen={startFilterOpen}
      onChange={onChipsChange}
    />
    <!-- Hidden input forwards the structured filter ids to the form action.
         The legacy comma-string `filters` field is still emitted in the
         page via $derived filtersString for back-compat. -->
    <input type="hidden" name="filter_item_ids" value={filterChips.map((f) => f.id).join(',')} />
  </div>
</section>

<style>
  .equip-head {
    margin-bottom: 16px;
  }
  .equip-setup {
    margin-bottom: 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .equip-applied {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 16px;
    background: rgba(232, 164, 58, 0.06);
    border: 1px solid var(--border-default);
    border-left: 2px solid var(--accent);
    border-radius: var(--r-sm);
    flex-wrap: wrap;
  }
  .equip-applied-square {
    width: 8px;
    height: 8px;
    background: var(--accent);
    flex: 0 0 8px;
  }
  .equip-applied-name {
    font-family: var(--font-display);
    font-style: italic;
    font-size: 16px;
    white-space: nowrap;
  }
  .equip-applied-summary {
    color: var(--fg-muted);
  }
  .equip-applied-spacer {
    flex: 1;
  }
  .equip-applied-id {
    color: var(--fg-faint);
  }
  .equip-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 24px;
    margin-bottom: 40px;
  }
  .equip-field--full {
    grid-column: 1 / -1;
  }
  .equip-filters {
    margin-bottom: 8px;
  }
  .equip-filters-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 12px;
  }
  .equip-filters-meta {
    color: var(--fg-faint);
  }
  @media (max-width: 640px) {
    .equip-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
