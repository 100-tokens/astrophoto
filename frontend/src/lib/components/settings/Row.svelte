<script lang="ts">
  let {
    label,
    hint,
    children
  }: {
    label: string;
    hint?: string;
    children: import('svelte').Snippet;
  } = $props();
</script>

<div class="row">
  <div class="label-cell">
    <span class="label">{label}</span>
    {#if hint}<span class="hint">{hint}</span>{/if}
  </div>
  <div class="field">{@render children()}</div>
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: 160px 1fr;
    gap: 24px;
    align-items: start;
    padding: 16px 0;
  }
  .label-cell {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .label {
    font-family: var(--font-mono);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    color: var(--fg-muted);
  }
  .hint {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-faint);
  }
</style>
