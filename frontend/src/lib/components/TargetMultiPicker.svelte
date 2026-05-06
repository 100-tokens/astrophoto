<script lang="ts">
  import TargetAutocompleteInput from './TargetAutocompleteInput.svelte';

  type Target = { slug: string; canonical_name: string; kind: string };

  interface Props {
    targets?: Target[];
    primary?: string | null;
    freetext?: string;
    max?: number;
  }
  let {
    targets = $bindable([]),
    primary = $bindable(null),
    freetext = $bindable(''),
    max = 5
  }: Props = $props();

  const excludeSlugs = $derived(targets.map((t) => t.slug));
  const atMax = $derived(targets.length >= max);

  function add(t: Target) {
    if (atMax) return;
    if (targets.some((x) => x.slug === t.slug)) return; // silent dedup
    targets = [...targets, t];
    if (primary === null) primary = t.slug;
  }

  function remove(slug: string) {
    targets = targets.filter((t) => t.slug !== slug);
    if (primary === slug) {
      primary = targets[0]?.slug ?? null;
    }
  }

  function promote(slug: string) {
    primary = slug;
  }
</script>

<div class="multi-picker">
  <div class="picker-label">
    <span class="t-label">Celestial subject(s)</span>
    <span class="t-meta counter">[{targets.length} / {max}]</span>
  </div>

  {#if targets.length}
    <ul class="target-chips" role="list">
      {#each targets as t (t.slug)}
        <li class="target-chip" class:is-primary={primary === t.slug}>
          <button
            type="button"
            class="promote-btn"
            onclick={() => promote(t.slug)}
            aria-label="Set as primary"
            aria-pressed={primary === t.slug}>★</button
          >
          <span class="chip-label">
            <span class="t-mono chip-slug">{t.slug.toUpperCase()}</span>
            <span class="chip-canonical">{t.canonical_name}</span>
          </span>
          <button
            type="button"
            class="remove-btn"
            onclick={() => remove(t.slug)}
            aria-label={`Remove ${t.canonical_name}`}>×</button
          >
        </li>
      {/each}
    </ul>
  {/if}

  {#if !atMax}
    <TargetAutocompleteInput placeholder="type to add an object…" {excludeSlugs} onPick={add} />
  {:else}
    <p class="t-meta hint-max">{max} subjects max — remove a chip to add another.</p>
  {/if}

  <div class="freetext-label">
    <span class="t-label">Or, free-text subject</span>
    <span class="t-meta hint-inline">(used only when no object is selected)</span>
  </div>
  <input
    type="text"
    bind:value={freetext}
    disabled={targets.length > 0}
    placeholder="e.g., summer Milky Way"
    class="input freetext-input"
  />
</div>

<style>
  .multi-picker {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .picker-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .counter {
    color: var(--fg-faint);
  }
  .target-chips {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .target-chip {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.6rem;
    background: var(--bg-raised);
    border: 1px solid var(--border-default);
    border-radius: var(--r-md);
  }
  .target-chip.is-primary {
    border-color: var(--accent);
    background: var(--bg-accent-tint);
  }
  .promote-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--fg-faint);
    font-size: 1.1rem;
    padding: 0 0.25rem;
    line-height: 1;
  }
  .target-chip.is-primary .promote-btn {
    color: var(--accent);
  }
  .chip-label {
    flex: 1;
    display: flex;
    gap: 0.5rem;
    align-items: baseline;
  }
  .chip-slug {
    font-weight: 600;
    color: var(--fg-primary);
  }
  .chip-canonical {
    color: var(--fg-muted);
    font-size: var(--t-sm);
  }
  .remove-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--fg-faint);
    font-size: 1.2rem;
    padding: 0 0.25rem;
    line-height: 1;
  }
  .remove-btn:hover {
    color: var(--fg-primary);
  }
  .hint-max {
    font-style: italic;
    margin: 0;
  }
  .hint-inline {
    font-weight: normal;
  }
  .freetext-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }
  .freetext-input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
