<script lang="ts">
  // Per-kind spec sub-form for focal modifiers (reducers, flatteners,
  // barlows, etc.). modifier_type / factor / self_weight_g are
  // required; backfocus and image circle are the completeness optionals.

  import type { FocalModifierSpecs } from '$lib/api/FocalModifierSpecs';

  type Props = {
    value: FocalModifierSpecs;
    disabled?: boolean;
    onChange: (next: FocalModifierSpecs) => void;
  };

  let { value, disabled = false, onChange }: Props = $props();

  function update(patch: Partial<FocalModifierSpecs>) {
    onChange({ ...value, ...patch });
  }

  function parseNum(s: string): number | null {
    const t = s.trim();
    if (t === '') return null;
    const n = Number(t);
    return Number.isFinite(n) ? n : null;
  }

  const MODIFIER_TYPE_OPTIONS = [
    { value: 'reducer', label: 'Reducer' },
    { value: 'flattener', label: 'Flattener' },
    { value: 'reducer_flattener', label: 'Reducer + flattener' },
    { value: 'barlow', label: 'Barlow' },
    { value: 'extender', label: 'Extender' },
    { value: 'corrector', label: 'Corrector' }
  ] as const;
</script>

<div class="ec-create-section">
  <label class="ec-create-row">
    <span class="ec-create-label is-required">Type</span>
    <select
      class="ec-create-input"
      value={value.modifier_type ?? ''}
      {disabled}
      onchange={(e) => {
        const v = (e.target as HTMLSelectElement).value;
        update({ modifier_type: (v || null) as FocalModifierSpecs['modifier_type'] });
      }}
    >
      <option value="" disabled>Pick a modifier type…</option>
      {#each MODIFIER_TYPE_OPTIONS as opt (opt.value)}
        <option value={opt.value}>{opt.label}</option>
      {/each}
    </select>
  </label>

  <label class="ec-create-row">
    <span class="ec-create-label is-required">Factor</span>
    <span class="ec-create-unit-wrap">
      <input
        class="ec-create-input"
        type="number"
        min="0.1"
        max="5"
        step="0.01"
        inputmode="decimal"
        placeholder="e.g. 0.79"
        value={value.factor ?? ''}
        {disabled}
        oninput={(e) => update({ factor: parseNum((e.target as HTMLInputElement).value) })}
      />
      <span class="ec-create-unit">×</span>
    </span>
  </label>

  <label class="ec-create-row">
    <span class="ec-create-label is-required">Self weight</span>
    <span class="ec-create-unit-wrap">
      <input
        class="ec-create-input"
        type="number"
        min="1"
        max="5000"
        step="1"
        inputmode="numeric"
        placeholder="e.g. 250"
        value={value.self_weight_g ?? ''}
        {disabled}
        oninput={(e) => update({ self_weight_g: parseNum((e.target as HTMLInputElement).value) })}
      />
      <span class="ec-create-unit">g</span>
    </span>
  </label>

  <details class="ec-create-details">
    <summary>More details · optional</summary>
    <div class="ec-create-details-body">
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
      <label class="ec-create-row">
        <span class="ec-create-label">Image circle</span>
        <span class="ec-create-unit-wrap">
          <input
            class="ec-create-input"
            type="number"
            min="0"
            max="200"
            step="0.1"
            inputmode="decimal"
            placeholder="optional"
            value={value.image_circle_mm ?? ''}
            {disabled}
            oninput={(e) =>
              update({ image_circle_mm: parseNum((e.target as HTMLInputElement).value) })}
          />
          <span class="ec-create-unit">mm</span>
        </span>
      </label>
    </div>
  </details>
</div>
