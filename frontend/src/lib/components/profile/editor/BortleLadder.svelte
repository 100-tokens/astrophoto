<script lang="ts">
  let {
    value = null,
    onChange
  }: {
    value?: number | null;
    onChange: (v: number | null) => void;
  } = $props();
</script>

<div class="ladder" role="radiogroup" aria-label="Bortle class">
  {#each Array.from({ length: 9 }, (_, i) => i + 1) as cell}
    <button
      type="button"
      role="radio"
      aria-checked={value === cell}
      class:selected={value === cell}
      onclick={() => onChange(value === cell ? null : cell)}
    >
      {cell}
    </button>
  {/each}
</div>

<style>
  .ladder {
    display: grid;
    grid-template-columns: repeat(9, 1fr);
    gap: 0;
    border: 1px solid var(--border-subtle);
  }
  .ladder button {
    background: transparent;
    color: var(--fg-secondary);
    border: 0;
    border-right: 1px solid var(--border-subtle);
    padding: 8px 0;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
  .ladder button:last-child {
    border-right: 0;
  }
  .ladder .selected {
    background: var(--accent);
    color: var(--accent-ink);
  }
</style>
