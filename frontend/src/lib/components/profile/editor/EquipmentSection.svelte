<script lang="ts">
  import type { EquipmentSummary } from '$lib/api/EquipmentSummary';
  import EquipmentAutocomplete from '$lib/components/EquipmentAutocomplete.svelte';

  let {
    equipment = { telescope: null, camera: null, mount: null, filters: null, guiding: null },
    onCommit
  }: {
    equipment?: EquipmentSummary;
    onCommit: (patch: { equipment: EquipmentSummary }) => Promise<void>;
  } = $props();

  let local = $state<EquipmentSummary>($state.snapshot(equipment));
  let saved = $state<EquipmentSummary>($state.snapshot(equipment));

  function changed(): boolean {
    // Compare normalised (trim + null) values so a trailing space doesn't
    // count as a change once it round-trips through commit.
    return JSON.stringify(normaliseAll(saved)) !== JSON.stringify(normaliseAll(local));
  }

  function norm(s: string | null | undefined): string | null {
    if (s == null) return null;
    const t = s.trim();
    return t === '' ? null : t;
  }

  function normaliseAll(eq: EquipmentSummary): EquipmentSummary {
    return {
      telescope: norm(eq.telescope),
      camera: norm(eq.camera),
      mount: norm(eq.mount),
      filters: norm(eq.filters),
      guiding: norm(eq.guiding)
    };
  }

  async function commit() {
    if (!changed()) return;
    const next = normaliseAll(local);
    await onCommit({ equipment: next });
    saved = $state.snapshot(next);
    local = $state.snapshot(next);
  }

  // Shape the EquipmentAutocomplete contract: it binds a string (not null),
  // and uses a `kind` enum where filters → 'filter' (singular). Glue
  // helpers translate between the bind:value strings and the nullable
  // EquipmentSummary fields.
  function get(key: keyof EquipmentSummary): string {
    return local[key] ?? '';
  }
  function set(key: keyof EquipmentSummary, v: string) {
    local = { ...local, [key]: norm(v) };
  }

  // Local controlled-binding shims so we can keep the existing
  // EquipmentAutocomplete bind:value contract while writing back into the
  // grouped EquipmentSummary.
  let scopeStr = $state(get('telescope'));
  let cameraStr = $state(get('camera'));
  let mountStr = $state(get('mount'));
  let filtersStr = $state(get('filters'));
  let guidingStr = $state(get('guiding'));

  $effect(() => {
    set('telescope', scopeStr);
  });
  $effect(() => {
    set('camera', cameraStr);
  });
  $effect(() => {
    set('mount', mountStr);
  });
  $effect(() => {
    set('filters', filtersStr);
  });
  $effect(() => {
    set('guiding', guidingStr);
  });
</script>

<fieldset class="section" onfocusout={() => void commit()}>
  <legend>Equipment</legend>
  <p class="hint">
    Start typing — we suggest gear other photographers (and you) have used so the same scope or
    camera doesn't end up under three different spellings.
  </p>
  <div class="field">
    <EquipmentAutocomplete name="profile-scope" kind="telescope" bind:value={scopeStr} />
  </div>
  <div class="field">
    <EquipmentAutocomplete name="profile-camera" kind="camera" bind:value={cameraStr} />
  </div>
  <div class="field">
    <EquipmentAutocomplete name="profile-mount" kind="mount" bind:value={mountStr} />
  </div>
  <div class="field">
    <EquipmentAutocomplete name="profile-filters" kind="filter" bind:value={filtersStr} />
  </div>
  <div class="field">
    <EquipmentAutocomplete name="profile-guiding" kind="guiding" bind:value={guidingStr} />
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
  .hint {
    margin: 0 0 12px;
    color: var(--fg-muted);
    font-size: 12px;
    line-height: 1.5;
  }
</style>
