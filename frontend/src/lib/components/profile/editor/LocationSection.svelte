<script lang="ts">
  import type { LocationSummary } from '$lib/api/LocationSummary';
  import BortleLadder from './BortleLadder.svelte';

  let {
    location = { location_text: null, bortle_class: null, sqm: null },
    onCommit
  }: {
    location?: LocationSummary;
    onCommit: (patch: { location: LocationSummary }) => Promise<void>;
  } = $props();

  let local = $state<LocationSummary>($state.snapshot(location));
  let saved = $state<LocationSummary>($state.snapshot(location));

  function changed(): boolean {
    return JSON.stringify(saved) !== JSON.stringify(local);
  }

  async function commit() {
    if (!changed()) return;
    await onCommit({ location: { ...local } });
    saved = $state.snapshot(local);
  }
</script>

<fieldset class="section" onfocusout={() => void commit()}>
  <legend>Location & sky</legend>
  <label class="field">
    <span>City / region</span>
    <input
      type="text"
      value={local.location_text ?? ''}
      oninput={(e) => {
        const v = (e.target as HTMLInputElement).value.trim();
        local = { ...local, location_text: v === '' ? null : v };
      }}
    />
  </label>
  <div class="field">
    <span>Bortle class</span>
    <BortleLadder
      value={local.bortle_class}
      onChange={(v) => {
        local = { ...local, bortle_class: v };
        void commit();
      }}
    />
  </div>
  <label class="field">
    <span>SQM (optional)</span>
    <input
      type="number"
      step="0.01"
      min="0"
      max="99.99"
      value={local.sqm ?? ''}
      oninput={(e) => {
        const v = (e.target as HTMLInputElement).valueAsNumber;
        local = { ...local, sqm: Number.isFinite(v) ? v : null };
      }}
    />
  </label>
</fieldset>

<style>
  .section {
    border: 1px solid var(--border-subtle);
    padding: 16px;
    margin: 0 0 16px;
  }
  legend {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-muted);
    padding: 0 6px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 12px;
  }
  .field span {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
  }
  .field input {
    background: var(--bg-canvas);
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 8px 10px;
    font-size: 14px;
  }
</style>
