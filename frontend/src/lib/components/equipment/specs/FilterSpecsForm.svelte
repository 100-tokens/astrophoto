<script lang="ts">
  // Per-kind spec sub-form for filters. Replaces the inline form that
  // previously lived inside FilterChipInput.svelte so the spec sub-form
  // for filters has a canonical home alongside the other *SpecsForm
  // components.
  //
  // REQUIRED: filter_type, bandwidth_nm (only when the type is
  // narrowband — `showBandwidth` predicate), mounted_diameter_mm.
  // OPTIONAL: thickness_mm, peak_transmission_pct (collapsed in fold).

  import type { FilterSpecs } from '$lib/api/FilterSpecs';
  import type { FilterType } from '$lib/api/FilterType';
  import { FILTER_TYPE_META } from '$lib/equipment/filter-types';

  type Props = {
    value: FilterSpecs;
    disabled?: boolean;
    onChange: (next: FilterSpecs) => void;
  };

  let { value, disabled = false, onChange }: Props = $props();

  function update(patch: Partial<FilterSpecs>) {
    onChange({ ...value, ...patch });
  }

  function parseNum(s: string): number | null {
    const t = s.trim();
    if (t === '') return null;
    const n = Number(t);
    return Number.isFinite(n) ? n : null;
  }

  // Iterating FILTER_TYPE_META gives us a single source of truth for
  // the select options. `showBandwidth` doubles as our
  // "narrowband-style → bandwidth_nm required" predicate.
  const FILTER_TYPE_OPTIONS = Object.entries(FILTER_TYPE_META).map(([value, meta]) => ({
    value: value as FilterType,
    label: meta.label
  }));

  const needsBandwidth = $derived(
    !!value.filter_type && FILTER_TYPE_META[value.filter_type].showBandwidth
  );
</script>

<div class="ec-create-section">
  <label class="ec-create-row">
    <span class="ec-create-label is-required">Type</span>
    <select
      class="ec-create-input"
      value={value.filter_type ?? ''}
      {disabled}
      onchange={(e) => {
        const v = (e.target as HTMLSelectElement).value;
        update({ filter_type: (v || null) as FilterSpecs['filter_type'] });
      }}
    >
      <option value="" disabled>Pick a filter type…</option>
      {#each FILTER_TYPE_OPTIONS as opt (opt.value)}
        <option value={opt.value}>{opt.label}</option>
      {/each}
    </select>
  </label>

  <label class="ec-create-row">
    <span class="ec-create-label" class:is-required={needsBandwidth}>
      Bandwidth
      {#if !needsBandwidth && value.filter_type}
        <span class="ec-create-hint">n/a</span>
      {/if}
    </span>
    <span class="ec-create-unit-wrap">
      <input
        class="ec-create-input"
        type="number"
        min="0"
        step="0.1"
        inputmode="decimal"
        placeholder={needsBandwidth ? 'e.g. 6' : '—'}
        value={value.bandwidth_nm ?? ''}
        disabled={!needsBandwidth || disabled}
        oninput={(e) => update({ bandwidth_nm: parseNum((e.target as HTMLInputElement).value) })}
      />
      <span class="ec-create-unit">nm</span>
    </span>
  </label>

  <label class="ec-create-row">
    <span class="ec-create-label is-required">Mounted Ø</span>
    <span class="ec-create-unit-wrap">
      <input
        class="ec-create-input"
        type="number"
        min="0"
        max="120"
        step="0.1"
        inputmode="decimal"
        placeholder="e.g. 50.8"
        value={value.mounted_diameter_mm ?? ''}
        {disabled}
        oninput={(e) =>
          update({ mounted_diameter_mm: parseNum((e.target as HTMLInputElement).value) })}
      />
      <span class="ec-create-unit">mm</span>
    </span>
  </label>

  <details class="ec-create-details">
    <summary>More details · optional</summary>
    <div class="ec-create-details-body">
      <label class="ec-create-row">
        <span class="ec-create-label">Thickness</span>
        <span class="ec-create-unit-wrap">
          <input
            class="ec-create-input"
            type="number"
            min="0"
            max="20"
            step="0.01"
            inputmode="decimal"
            placeholder="optional"
            value={value.thickness_mm ?? ''}
            {disabled}
            oninput={(e) =>
              update({ thickness_mm: parseNum((e.target as HTMLInputElement).value) })}
          />
          <span class="ec-create-unit">mm</span>
        </span>
      </label>
      <label class="ec-create-row">
        <span class="ec-create-label">Peak transmission</span>
        <span class="ec-create-unit-wrap">
          <input
            class="ec-create-input"
            type="number"
            min="0"
            max="100"
            step="0.1"
            inputmode="decimal"
            placeholder="optional"
            value={value.peak_transmission_pct ?? ''}
            {disabled}
            oninput={(e) =>
              update({ peak_transmission_pct: parseNum((e.target as HTMLInputElement).value) })}
          />
          <span class="ec-create-unit">%</span>
        </span>
      </label>
    </div>
  </details>
</div>
