<script lang="ts">
  import { untrack } from 'svelte';
  import type { EquipmentSummary } from '$lib/api/EquipmentSummary';
  import EquipmentAutocomplete from '$lib/components/EquipmentAutocomplete.svelte';

  let {
    equipment = { telescope: null, camera: null, mount: null, filters: null, guiding: null },
    onCommit
  }: {
    equipment?: EquipmentSummary;
    onCommit: (patch: { equipment: EquipmentSummary }) => Promise<void>;
  } = $props();

  // Each field is its own $state — we drop the "local: EquipmentSummary"
  // mirror because writing it inside an $effect that also reads it caused
  // Svelte's effect_update_depth_exceeded bail-out (which silently killed
  // the EquipmentAutocomplete's own effect, so no suggestions ever fetched).
  // untrack() declares the prop read is a one-time seed.
  let scope = $state<string>(untrack(() => equipment.telescope ?? ''));
  let camera = $state<string>(untrack(() => equipment.camera ?? ''));
  let mount = $state<string>(untrack(() => equipment.mount ?? ''));
  let filters = $state<string>(untrack(() => equipment.filters ?? ''));
  let guiding = $state<string>(untrack(() => equipment.guiding ?? ''));

  let saved = $state<EquipmentSummary>(untrack(() => $state.snapshot(equipment)));

  function norm(s: string): string | null {
    const t = s.trim();
    return t === '' ? null : t;
  }

  function current(): EquipmentSummary {
    return {
      telescope: norm(scope),
      camera: norm(camera),
      mount: norm(mount),
      filters: norm(filters),
      guiding: norm(guiding)
    };
  }

  async function commit() {
    const next = current();
    if (JSON.stringify(saved) === JSON.stringify(next)) return;
    await onCommit({ equipment: next });
    saved = next;
  }
</script>

<fieldset class="section" onfocusout={() => void commit()}>
  <legend>Equipment</legend>
  <p class="hint">
    Start typing — we suggest gear other photographers (and you) have used so the same scope or
    camera doesn't end up under three different spellings.
  </p>
  <div class="field">
    <EquipmentAutocomplete name="profile-scope" kind="telescope" bind:value={scope} />
  </div>
  <div class="field">
    <EquipmentAutocomplete name="profile-camera" kind="camera" bind:value={camera} />
  </div>
  <div class="field">
    <EquipmentAutocomplete name="profile-mount" kind="mount" bind:value={mount} />
  </div>
  <div class="field">
    <EquipmentAutocomplete name="profile-filters" kind="filter" bind:value={filters} />
  </div>
  <div class="field">
    <EquipmentAutocomplete name="profile-guiding" kind="guiding" bind:value={guiding} />
  </div>
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
  .hint {
    margin: 0 0 12px;
    color: var(--fg-muted);
    font-size: 12px;
    line-height: 1.5;
  }
</style>
