<script lang="ts">
  import type { LocationSummary } from '$lib/api/LocationSummary';

  let {
    location,
    isOwner,
    onEditProfile
  }: {
    location: LocationSummary;
    isOwner: boolean;
    onEditProfile: () => void;
  } = $props();

  let parts = $derived(
    [
      location.location_text,
      location.bortle_class != null ? `Bortle ${location.bortle_class}` : null,
      location.sqm != null ? `SQM ${location.sqm.toFixed(2)}` : null
    ].filter(Boolean) as string[]
  );
</script>

{#if parts.length > 0}
  <section class="badge">
    {#each parts as p, i}
      <span>{p}</span>
      {#if i < parts.length - 1}<span class="dot">·</span>{/if}
    {/each}
  </section>
{:else if isOwner}
  <section class="badge empty">
    <button type="button" class="prompt" onclick={onEditProfile}>
      Where do you observe from?
    </button>
  </section>
{/if}

<style>
  .badge {
    padding: 12px 32px;
    border-top: 1px solid var(--border-subtle);
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-secondary);
    display: flex;
    gap: 8px;
  }
  .badge .dot {
    color: var(--fg-muted);
  }
  .empty .prompt {
    background: transparent;
    color: var(--accent);
    border: 1px dashed var(--border-subtle);
    padding: 8px 16px;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
</style>
