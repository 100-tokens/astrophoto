<script lang="ts">
  // Per-kind spec sub-form for mounts. mount_type / payload_kg /
  // self_weight_kg / goto are the catalog-grade required fields;
  // periodic error / tripod / control protocol are optional.

  import type { MountSpecs } from '$lib/api/MountSpecs';

  type Props = {
    value: MountSpecs;
    disabled?: boolean;
    onChange: (next: MountSpecs) => void;
  };

  let { value, disabled = false, onChange }: Props = $props();

  function update(patch: Partial<MountSpecs>) {
    onChange({ ...value, ...patch });
  }

  function parseNum(s: string): number | null {
    const t = s.trim();
    if (t === '') return null;
    const n = Number(t);
    return Number.isFinite(n) ? n : null;
  }

  const MOUNT_TYPE_OPTIONS = [
    { value: 'equatorial_german', label: 'Equatorial German' },
    { value: 'equatorial_fork', label: 'Equatorial fork' },
    { value: 'alt_az', label: 'Alt-Az' },
    { value: 'harmonic_drive', label: 'Harmonic drive' },
    { value: 'strain_wave', label: 'Strain wave' },
    { value: 'other', label: 'Other' }
  ] as const;
</script>

<div class="ec-create-section">
  <label class="ec-create-row">
    <span class="ec-create-label is-required">Type</span>
    <select
      class="ec-create-input"
      value={value.mount_type ?? ''}
      {disabled}
      onchange={(e) => {
        const v = (e.target as HTMLSelectElement).value;
        update({ mount_type: (v || null) as MountSpecs['mount_type'] });
      }}
    >
      <option value="" disabled>Pick a mount type…</option>
      {#each MOUNT_TYPE_OPTIONS as opt (opt.value)}
        <option value={opt.value}>{opt.label}</option>
      {/each}
    </select>
  </label>

  <label class="ec-create-row">
    <span class="ec-create-label is-required">Payload</span>
    <span class="ec-create-unit-wrap">
      <input
        class="ec-create-input"
        type="number"
        min="0.5"
        max="200"
        step="0.1"
        inputmode="decimal"
        placeholder="e.g. 20"
        value={value.payload_kg ?? ''}
        {disabled}
        oninput={(e) => update({ payload_kg: parseNum((e.target as HTMLInputElement).value) })}
      />
      <span class="ec-create-unit">kg</span>
    </span>
  </label>

  <label class="ec-create-row">
    <span class="ec-create-label is-required">Self weight</span>
    <span class="ec-create-unit-wrap">
      <input
        class="ec-create-input"
        type="number"
        min="0.5"
        max="200"
        step="0.1"
        inputmode="decimal"
        placeholder="e.g. 15"
        value={value.self_weight_kg ?? ''}
        {disabled}
        oninput={(e) => update({ self_weight_kg: parseNum((e.target as HTMLInputElement).value) })}
      />
      <span class="ec-create-unit">kg</span>
    </span>
  </label>

  <label class="ec-create-row">
    <span class="ec-create-label is-required">GoTo</span>
    <span class="ec-create-bool-wrap">
      <input
        type="checkbox"
        checked={value.goto === true}
        {disabled}
        onchange={(e) => update({ goto: (e.target as HTMLInputElement).checked })}
      />
      <span class="ec-create-hint">tick if the mount has GoTo</span>
    </span>
  </label>

  <details class="ec-create-details">
    <summary>More details · optional</summary>
    <div class="ec-create-details-body">
      <label class="ec-create-row">
        <span class="ec-create-label">Periodic error</span>
        <span class="ec-create-unit-wrap">
          <input
            class="ec-create-input"
            type="number"
            min="0"
            step="0.1"
            inputmode="decimal"
            placeholder="optional"
            value={value.periodic_error_arcsec ?? ''}
            {disabled}
            oninput={(e) =>
              update({ periodic_error_arcsec: parseNum((e.target as HTMLInputElement).value) })}
          />
          <span class="ec-create-unit">arcsec</span>
        </span>
      </label>
      <label class="ec-create-row">
        <span class="ec-create-label">Tripod incl.</span>
        <span class="ec-create-bool-wrap">
          <input
            type="checkbox"
            checked={value.tripod_included === true}
            {disabled}
            onchange={(e) => update({ tripod_included: (e.target as HTMLInputElement).checked })}
          />
          <span class="ec-create-hint">bundled in self-weight</span>
        </span>
      </label>
      <label class="ec-create-row">
        <span class="ec-create-label">Control protocol</span>
        <input
          class="ec-create-input"
          placeholder="e.g. synscan, nexstar, onstep, ascom_native"
          autocomplete="off"
          spellcheck="false"
          value={value.control_protocol ?? ''}
          {disabled}
          oninput={(e) => {
            const v = (e.target as HTMLInputElement).value.trim();
            update({ control_protocol: v || null });
          }}
        />
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
