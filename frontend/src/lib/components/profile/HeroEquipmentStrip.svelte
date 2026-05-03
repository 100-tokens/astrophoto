<script lang="ts">
  import type { EquipmentSummary } from '$lib/api/EquipmentSummary';

  let {
    equipment,
    isOwner,
    onEditProfile
  }: {
    equipment: EquipmentSummary;
    isOwner: boolean;
    onEditProfile: () => void;
  } = $props();

  let cells = $derived(
    [
      ['SCOPE', equipment.telescope],
      ['CAM', equipment.camera],
      ['MOUNT', equipment.mount],
      ['FILTERS', equipment.filters],
      ['GUIDING', equipment.guiding]
    ].filter(([, v]) => v != null && v.trim() !== '') as [string, string][]
  );
</script>

{#if cells.length > 0}
  <section class="strip">
    {#each cells as [label, value]}
      <div class="cell"><span class="lab">{label}</span> &nbsp; {value}</div>
    {/each}
  </section>
{:else if isOwner}
  <section class="strip empty">
    <button type="button" class="prompt" onclick={onEditProfile}>
      Add the gear behind your shots
    </button>
  </section>
{/if}

<style>
  .strip {
    padding: 16px 32px;
    border-top: 1px solid var(--border-subtle);
    display: flex;
    flex-wrap: wrap;
    gap: 24px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-secondary);
  }
  .lab {
    color: var(--fg-muted);
  }
  .empty .prompt {
    background: transparent;
    color: var(--accent);
    border: 1px dashed var(--border-subtle);
    padding: 12px 16px;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
</style>
