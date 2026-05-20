<script lang="ts">
  // Per-kind spec sub-form for guiding setups (camera + OAG / guidescope).
  // setup_kind / guide_focal_mm / guide_aperture_mm are required;
  // guide_camera is the free-text optional. This matches the v2
  // schema (migration 0022 — guiding_specs).

  import type { GuidingSpecs } from '$lib/api/GuidingSpecs';

  type Props = {
    value: GuidingSpecs;
    disabled?: boolean;
    onChange: (next: GuidingSpecs) => void;
  };

  let { value, disabled = false, onChange }: Props = $props();

  function update(patch: Partial<GuidingSpecs>) {
    onChange({ ...value, ...patch });
  }

  function parseNum(s: string): number | null {
    const t = s.trim();
    if (t === '') return null;
    const n = Number(t);
    return Number.isFinite(n) ? n : null;
  }

  const SETUP_KIND_OPTIONS = [
    { value: 'oag', label: 'Off-axis guider (OAG)' },
    { value: 'guidescope', label: 'Guide scope' },
    { value: 'oag_prism', label: 'OAG with prism' },
    { value: 'other', label: 'Other' }
  ] as const;
</script>

<div class="ec-create-section">
  <label class="ec-create-row">
    <span class="ec-create-label is-required">Setup kind</span>
    <select
      class="ec-create-input"
      value={value.setup_kind ?? ''}
      {disabled}
      onchange={(e) => {
        const v = (e.target as HTMLSelectElement).value;
        update({ setup_kind: (v || null) as GuidingSpecs['setup_kind'] });
      }}
    >
      <option value="" disabled>Pick a setup kind…</option>
      {#each SETUP_KIND_OPTIONS as opt (opt.value)}
        <option value={opt.value}>{opt.label}</option>
      {/each}
    </select>
  </label>

  <label class="ec-create-row">
    <span class="ec-create-label is-required">Guide focal</span>
    <span class="ec-create-unit-wrap">
      <input
        class="ec-create-input"
        type="number"
        min="1"
        max="1000"
        step="1"
        inputmode="numeric"
        placeholder="e.g. 240"
        value={value.guide_focal_mm ?? ''}
        {disabled}
        oninput={(e) => update({ guide_focal_mm: parseNum((e.target as HTMLInputElement).value) })}
      />
      <span class="ec-create-unit">mm</span>
    </span>
  </label>

  <label class="ec-create-row">
    <span class="ec-create-label is-required">Guide aperture</span>
    <span class="ec-create-unit-wrap">
      <input
        class="ec-create-input"
        type="number"
        min="1"
        max="300"
        step="1"
        inputmode="numeric"
        placeholder="e.g. 60"
        value={value.guide_aperture_mm ?? ''}
        {disabled}
        oninput={(e) =>
          update({ guide_aperture_mm: parseNum((e.target as HTMLInputElement).value) })}
      />
      <span class="ec-create-unit">mm</span>
    </span>
  </label>

  <details class="ec-create-details">
    <summary>More details · optional</summary>
    <div class="ec-create-details-body">
      <label class="ec-create-row">
        <span class="ec-create-label">Guide camera</span>
        <input
          class="ec-create-input"
          placeholder="e.g. ZWO ASI120MM Mini"
          autocomplete="off"
          spellcheck="false"
          value={value.guide_camera ?? ''}
          {disabled}
          oninput={(e) => {
            const v = (e.target as HTMLInputElement).value.trim();
            update({ guide_camera: v || null });
          }}
        />
      </label>
    </div>
  </details>
</div>
