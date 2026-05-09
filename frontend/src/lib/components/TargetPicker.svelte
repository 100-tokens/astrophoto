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

  // Fallback: the user typed something that didn't match any catalog entry
  // (e.g. "M42 Orion Nebula" before the OpenNGC catalog had it, or a personal
  // target name like "Backyard moon"). Keep the typed text as the target so
  // the autosave/PUT carries it. Server-side `attach_primary_by_freetext` then
  // resolves against the catalog by slug/alias/canonical_name; misses are
  // recorded as freetext only (no joined target row), which is the right
  // behaviour for novel targets.
  function handleFreetext(text: string) {
    const trimmed = text.trim();
    if (trimmed) value = trimmed;
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
  <TargetAutocompleteInput
    id={name}
    {api}
    placeholder="M31, NGC 7000…"
    onPick={handlePick}
    onFreetextCommit={handleFreetext}
  />
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
