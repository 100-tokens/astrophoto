<script lang="ts">
  // Per-kind spec sub-form for catalog v2 saisie-forcée. Renders REQUIRED
  // fields (design, aperture, focal length, self-weight) above the fold
  // with the accent-dot label marker; OPTIONAL completeness fields
  // (optical length, backfocus) inside a collapsible <details>.
  //
  // Pure controlled component — the parent owns the value and onChange.
  // The shape returned via onChange matches the `kind: "telescope"`
  // variant of EquipmentSpecsPayload exactly (see backend/api_types.rs).
  // Computed fields like focal_ratio_f are NOT emitted — the DB
  // recomputes them as STORED columns.

  import type { TelescopeSpecs } from '$lib/api/TelescopeSpecs';

  type Props = {
    value: TelescopeSpecs;
    disabled?: boolean;
    onChange: (next: TelescopeSpecs) => void;
  };

  let { value, disabled = false, onChange }: Props = $props();

  function update(patch: Partial<TelescopeSpecs>) {
    onChange({ ...value, ...patch });
  }

  // Generic number parser: empty / NaN → null, so the parent sees
  // missing values consistently. Avoid coercing 0 to null (legitimate
  // value for some fields).
  function parseNum(s: string): number | null {
    const t = s.trim();
    if (t === '') return null;
    const n = Number(t);
    return Number.isFinite(n) ? n : null;
  }

  const DESIGN_OPTIONS = [
    { value: 'refractor_apo', label: 'Refractor APO' },
    { value: 'refractor_achro', label: 'Refractor achro' },
    { value: 'sct', label: 'SCT' },
    { value: 'rc', label: 'RC' },
    { value: 'newtonian', label: 'Newtonian' },
    { value: 'maksutov_cassegrain', label: 'Maksutov-Cassegrain' },
    { value: 'maksutov_newtonian', label: 'Maksutov-Newtonian' },
    { value: 'dall_kirkham', label: 'Dall-Kirkham' },
    { value: 'other', label: 'Other' }
  ] as const;
</script>

<div class="ec-create-section">
  <label class="ec-create-row">
    <span class="ec-create-label is-required">Design</span>
    <select
      class="ec-create-input"
      value={value.design ?? ''}
      {disabled}
      onchange={(e) => {
        const v = (e.target as HTMLSelectElement).value;
        update({ design: (v || null) as TelescopeSpecs['design'] });
      }}
    >
      <option value="" disabled>Pick a design…</option>
      {#each DESIGN_OPTIONS as opt (opt.value)}
        <option value={opt.value}>{opt.label}</option>
      {/each}
    </select>
  </label>

  <label class="ec-create-row">
    <span class="ec-create-label is-required">Aperture</span>
    <span class="ec-create-unit-wrap">
      <input
        class="ec-create-input"
        type="number"
        min="30"
        max="1500"
        step="1"
        inputmode="numeric"
        placeholder="e.g. 100"
        value={value.aperture_mm ?? ''}
        {disabled}
        oninput={(e) => update({ aperture_mm: parseNum((e.target as HTMLInputElement).value) })}
      />
      <span class="ec-create-unit">mm</span>
    </span>
  </label>

  <label class="ec-create-row">
    <span class="ec-create-label is-required">Focal length</span>
    <span class="ec-create-unit-wrap">
      <input
        class="ec-create-input"
        type="number"
        min="100"
        max="15000"
        step="1"
        inputmode="numeric"
        placeholder="e.g. 550"
        value={value.focal_length_mm ?? ''}
        {disabled}
        oninput={(e) => update({ focal_length_mm: parseNum((e.target as HTMLInputElement).value) })}
      />
      <span class="ec-create-unit">mm</span>
    </span>
  </label>

  <label class="ec-create-row">
    <span class="ec-create-label is-required">Self weight</span>
    <span class="ec-create-unit-wrap">
      <input
        class="ec-create-input"
        type="number"
        min="0.1"
        max="200"
        step="0.1"
        inputmode="decimal"
        placeholder="e.g. 5.5"
        value={value.self_weight_kg ?? ''}
        {disabled}
        oninput={(e) => update({ self_weight_kg: parseNum((e.target as HTMLInputElement).value) })}
      />
      <span class="ec-create-unit">kg</span>
    </span>
  </label>

  <details class="ec-create-details">
    <summary>More details · optional</summary>
    <div class="ec-create-details-body">
      <label class="ec-create-row">
        <span class="ec-create-label">Optical length</span>
        <span class="ec-create-unit-wrap">
          <input
            class="ec-create-input"
            type="number"
            min="50"
            max="3000"
            step="1"
            inputmode="numeric"
            placeholder="optional"
            value={value.optical_length_mm ?? ''}
            {disabled}
            oninput={(e) =>
              update({ optical_length_mm: parseNum((e.target as HTMLInputElement).value) })}
          />
          <span class="ec-create-unit">mm</span>
        </span>
      </label>
      <label class="ec-create-row">
        <span class="ec-create-label">Backfocus</span>
        <span class="ec-create-unit-wrap">
          <input
            class="ec-create-input"
            type="number"
            min="0"
            max="500"
            step="0.1"
            inputmode="decimal"
            placeholder="optional"
            value={value.backfocus_mm ?? ''}
            {disabled}
            oninput={(e) =>
              update({ backfocus_mm: parseNum((e.target as HTMLInputElement).value) })}
          />
          <span class="ec-create-unit">mm</span>
        </span>
      </label>
    </div>
  </details>
</div>
