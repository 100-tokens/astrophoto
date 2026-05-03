<script lang="ts">
  import type { EquipmentSummary } from '$lib/api/EquipmentSummary';

  let {
    equipment = { telescope: null, camera: null, mount: null, filters: null, guiding: null },
    onCommit
  }: {
    equipment?: EquipmentSummary;
    onCommit: (patch: { equipment: EquipmentSummary }) => Promise<void>;
  } = $props();

  let local = $state<EquipmentSummary>(structuredClone(equipment));
  let saved = $state<EquipmentSummary>(structuredClone(equipment));

  function changed(): boolean {
    return JSON.stringify(saved) !== JSON.stringify(local);
  }

  async function commit() {
    if (!changed()) return;
    await onCommit({ equipment: { ...local } });
    saved = structuredClone(local);
  }

  function norm(s: string): string | null {
    const t = s.trim();
    return t === '' ? null : t;
  }

  const FIELDS: Array<[string, keyof EquipmentSummary]> = [
    ['Scope', 'telescope'],
    ['Camera', 'camera'],
    ['Mount', 'mount'],
    ['Filters', 'filters'],
    ['Guiding', 'guiding']
  ];
</script>

<fieldset class="section" onfocusout={() => void commit()}>
  <legend>Equipment</legend>
  {#each FIELDS as [label, key]}
    <label class="field">
      <span>{label}</span>
      <input
        type="text"
        value={local[key] ?? ''}
        oninput={(e) => {
          local = { ...local, [key]: norm((e.target as HTMLInputElement).value) };
        }}
      />
    </label>
  {/each}
</fieldset>

<style>
  .section {
    border: 1px solid var(--border-subtle);
    padding: 16px;
    margin: 0 0 16px;
  }
  legend { font-family: var(--font-mono); font-size: 12px; color: var(--fg-muted); padding: 0 6px; }
  .field { display: flex; flex-direction: column; gap: 6px; margin-bottom: 12px; }
  .field span { font-family: var(--font-mono); font-size: 11px; color: var(--fg-muted); }
  .field input {
    background: var(--bg-canvas);
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 8px 10px;
    font-size: 14px;
  }
</style>
