<script lang="ts">
  interface Props {
    name?: string;
    value?: string;
  }

  let { name = 'category', value = $bindable('other') }: Props = $props();

  const opts = ['dso', 'planetary', 'lunar', 'solar', 'wide_field', 'nightscape', 'other'] as const;
</script>

<!-- The hidden input carries the value; visible label is purely decorative for this segmented control -->
<!-- svelte-ignore a11y_label_has_associated_control -->
<label class="t-label">CATEGORY</label>
<div class="seg" role="radiogroup" aria-label="Category">
  {#each opts as o}
    <button
      type="button"
      class="seg-btn"
      class:active={value === o}
      role="radio"
      aria-checked={value === o ? 'true' : 'false'}
      onclick={() => (value = o)}
    >
      {o.replace('_', ' ')}
    </button>
  {/each}
  <input type="hidden" {name} {value} />
</div>

<style>
  .seg {
    display: flex;
    border: 1px solid var(--border-default);
  }
  .seg-btn {
    padding: 6px 12px;
    font-family: var(--font-mono);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--fg-secondary);
    border-right: 1px solid var(--border-default);
    background: none;
    cursor: pointer;
    border-top: none;
    border-bottom: none;
    border-left: none;
  }
  .seg-btn:last-of-type {
    border-right: none;
  }
  .seg-btn.active {
    background: var(--accent);
    color: var(--accent-ink);
  }
</style>
