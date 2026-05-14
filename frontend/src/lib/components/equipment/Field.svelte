<script lang="ts">
  import type { Snippet } from 'svelte';

  type Props = {
    label: string;
    value?: string;
    mono?: boolean;
    detected?: boolean | 'auto';
    hint?: string;
    full?: boolean;
    children?: Snippet;
  };

  let { label, value = '', mono = false, detected, hint, full = false, children }: Props = $props();

  const showDetected = $derived(detected === true);
  const showAuto = $derived(detected === 'auto');
</script>

<div class="field" class:is-full={full}>
  <div class="field-head">
    <span class="t-label">{label}</span>
    {#if showDetected}
      <span class="field-meta field-meta--detected">● DETECTED FROM EXIF</span>
    {:else if showAuto}
      <span class="field-meta field-meta--auto">○ AUTO-FILL</span>
    {/if}
  </div>
  {#if children}
    {@render children()}
  {:else}
    <input class="input" class:input-mono={mono} {value} />
  {/if}
  {#if hint}
    <span class="field-hint t-meta">{hint}</span>
  {/if}
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .is-full {
    grid-column: 1 / -1;
  }

  .field-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .field-meta {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .field-meta--detected {
    color: var(--accent);
  }

  .field-meta--auto {
    color: var(--fg-faint);
  }

  .field-hint {
    color: var(--fg-muted);
    font-size: 11px;
  }
</style>
