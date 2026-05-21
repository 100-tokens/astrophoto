<script lang="ts">
  // Per-kind spec sub-form for cameras. Same shape contract as the other
  // *SpecsForm components: REQUIRED fields above the fold, OPTIONAL
  // completeness fields tucked into a <details> fold.

  import type { CameraSpecs } from '$lib/api/CameraSpecs';

  type Props = {
    value: CameraSpecs;
    disabled?: boolean;
    onChange: (next: CameraSpecs) => void;
  };

  let { value, disabled = false, onChange }: Props = $props();

  function update(patch: Partial<CameraSpecs>) {
    onChange({ ...value, ...patch });
  }

  function parseNum(s: string): number | null {
    const t = s.trim();
    if (t === '') return null;
    const n = Number(t);
    return Number.isFinite(n) ? n : null;
  }

  const SENSOR_TYPE_OPTIONS = [
    { value: 'cmos', label: 'CMOS' },
    { value: 'ccd', label: 'CCD' }
  ] as const;

  const COLOR_TYPE_OPTIONS = [
    { value: 'mono', label: 'Mono' },
    { value: 'osc', label: 'OSC (one-shot color)' }
  ] as const;
</script>

<div class="ec-create-section">
  <label class="ec-create-row">
    <span class="ec-create-label is-required">Sensor type</span>
    <select
      class="ec-create-input"
      value={value.sensor_type ?? ''}
      {disabled}
      onchange={(e) => {
        const v = (e.target as HTMLSelectElement).value;
        update({ sensor_type: (v || null) as CameraSpecs['sensor_type'] });
      }}
    >
      <option value="" disabled>Pick a sensor type…</option>
      {#each SENSOR_TYPE_OPTIONS as opt (opt.value)}
        <option value={opt.value}>{opt.label}</option>
      {/each}
    </select>
  </label>

  <label class="ec-create-row">
    <span class="ec-create-label is-required">Color type</span>
    <select
      class="ec-create-input"
      value={value.color_type ?? ''}
      {disabled}
      onchange={(e) => {
        const v = (e.target as HTMLSelectElement).value;
        update({ color_type: (v || null) as CameraSpecs['color_type'] });
      }}
    >
      <option value="" disabled>Pick a color type…</option>
      {#each COLOR_TYPE_OPTIONS as opt (opt.value)}
        <option value={opt.value}>{opt.label}</option>
      {/each}
    </select>
  </label>

  <label class="ec-create-row">
    <span class="ec-create-label is-required">Cooled</span>
    <span class="ec-create-bool-wrap">
      <input
        type="checkbox"
        checked={value.cooled === true}
        {disabled}
        onchange={(e) => update({ cooled: (e.target as HTMLInputElement).checked })}
      />
      <span class="ec-create-hint">tick if active TEC cooling</span>
    </span>
  </label>

  <label class="ec-create-row">
    <span class="ec-create-label is-required">Sensor model</span>
    <input
      class="ec-create-input"
      placeholder="e.g. IMX571"
      autocomplete="off"
      spellcheck="false"
      value={value.sensor_model ?? ''}
      {disabled}
      oninput={(e) => {
        const v = (e.target as HTMLInputElement).value.trim();
        update({ sensor_model: v || null });
      }}
    />
  </label>

  <label class="ec-create-row">
    <span class="ec-create-label is-required">Pixel size</span>
    <span class="ec-create-unit-wrap">
      <input
        class="ec-create-input"
        type="number"
        min="0.5"
        max="25"
        step="0.01"
        inputmode="decimal"
        placeholder="e.g. 3.76"
        value={value.pixel_size_um ?? ''}
        {disabled}
        oninput={(e) => update({ pixel_size_um: parseNum((e.target as HTMLInputElement).value) })}
      />
      <span class="ec-create-unit">µm</span>
    </span>
  </label>

  <label class="ec-create-row">
    <span class="ec-create-label is-required">Sensor width</span>
    <span class="ec-create-unit-wrap">
      <input
        class="ec-create-input"
        type="number"
        min="1"
        step="1"
        inputmode="numeric"
        placeholder="e.g. 6248"
        value={value.sensor_width_px ?? ''}
        {disabled}
        oninput={(e) => update({ sensor_width_px: parseNum((e.target as HTMLInputElement).value) })}
      />
      <span class="ec-create-unit">px</span>
    </span>
  </label>

  <label class="ec-create-row">
    <span class="ec-create-label is-required">Sensor height</span>
    <span class="ec-create-unit-wrap">
      <input
        class="ec-create-input"
        type="number"
        min="1"
        step="1"
        inputmode="numeric"
        placeholder="e.g. 4176"
        value={value.sensor_height_px ?? ''}
        {disabled}
        oninput={(e) =>
          update({ sensor_height_px: parseNum((e.target as HTMLInputElement).value) })}
      />
      <span class="ec-create-unit">px</span>
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
        placeholder="e.g. 700"
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
        <span class="ec-create-label">Full-well</span>
        <span class="ec-create-unit-wrap">
          <input
            class="ec-create-input"
            type="number"
            min="0"
            step="100"
            inputmode="numeric"
            placeholder="optional"
            value={value.full_well_capacity_e ?? ''}
            {disabled}
            oninput={(e) =>
              update({ full_well_capacity_e: parseNum((e.target as HTMLInputElement).value) })}
          />
          <span class="ec-create-unit">e⁻</span>
        </span>
      </label>
      <label class="ec-create-row">
        <span class="ec-create-label">Read noise</span>
        <span class="ec-create-unit-wrap">
          <input
            class="ec-create-input"
            type="number"
            min="0"
            step="0.01"
            inputmode="decimal"
            placeholder="optional"
            value={value.read_noise_e ?? ''}
            {disabled}
            oninput={(e) =>
              update({ read_noise_e: parseNum((e.target as HTMLInputElement).value) })}
          />
          <span class="ec-create-unit">e⁻</span>
        </span>
      </label>
      <label class="ec-create-row">
        <span class="ec-create-label">Mount thread</span>
        <input
          class="ec-create-input"
          placeholder="e.g. T2, M48, M54, EF"
          autocomplete="off"
          spellcheck="false"
          value={value.mount_thread ?? ''}
          {disabled}
          oninput={(e) => {
            const v = (e.target as HTMLInputElement).value.trim();
            update({ mount_thread: v || null });
          }}
        />
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

<style>
  .ec-create-bool-wrap {
    display: inline-flex;
    align-items: center;
    gap: var(--s-2);
    color: var(--fg-secondary);
    font-size: var(--t-sm);
  }
  .ec-create-bool-wrap input[type='checkbox'] {
    accent-color: var(--accent);
  }
</style>
