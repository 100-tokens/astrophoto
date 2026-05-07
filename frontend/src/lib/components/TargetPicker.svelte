<script lang="ts">
  import TargetAutocompleteInput from './TargetAutocompleteInput.svelte';

  type Target = { slug: string; canonical_name: string; kind: string };

  interface Props {
    name?: string;
    value?: string;
    api?: string;
  }
  let {
    name = 'target',
    value = $bindable(''),
    api = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? ''
  }: Props = $props();

  function handlePick(t: Target) {
    value = t.canonical_name;
  }
</script>

<label class="t-label" for={name}>TARGET</label>
<input type="hidden" {name} {value} />
{#if value}
  <div class="selected-chip">
    <span class="t-mono">{value}</span>
    <button type="button" class="clear-btn" aria-label="Clear target" onclick={() => (value = '')}
      >&times;</button
    >
  </div>
{:else}
  <TargetAutocompleteInput id={name} {api} placeholder="M31, NGC 7000…" onPick={handlePick} />
{/if}

<style>
  .selected-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 4px 10px;
    background: var(--bg-elevated);
    border: 1px solid var(--border, #444);
    border-radius: 4px;
    font-size: 0.875rem;
  }
  .clear-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-muted, #888);
    font-size: 1rem;
    line-height: 1;
    padding: 0;
  }
  .clear-btn:hover {
    color: var(--text, #fff);
  }
</style>
