<script lang="ts">
  import type { EquipmentPaired } from '$lib/api/EquipmentPaired';

  let { items }: { items: EquipmentPaired[] } = $props();

  let visible = $derived(items.slice(0, 4));
</script>

{#if visible.length > 0}
  <section class="paired-rail">
    <!-- Handoff calls this "Other <brand>" siblings (same canonical brand prefix);
         current backend returns co-used items instead. Label kept honest. -->
    <p class="rail-label">Often paired with</p>
    <div class="chips">
      {#each visible as item (item.kind + '/' + item.slug)}
        <a href="/equip/{item.kind}/{item.slug}" class="chip">
          <span class="chip-name">{item.display_name}</span>
          <span class="chip-count">{Number(item.shared_count).toLocaleString('en-US')}</span>
        </a>
      {/each}
    </div>
  </section>
{/if}

<style>
  .paired-rail {
    padding: 24px 64px;
    border-top: 1px solid var(--border-subtle);
  }

  .rail-label {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.06em;
    color: var(--fg-muted);
    text-transform: uppercase;
    margin: 0 0 12px 0;
  }

  .chips {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border: 1px solid var(--border-default);
    color: var(--fg-secondary);
    text-decoration: none;
    font-family: var(--font-mono);
    font-size: 12px;
    transition:
      border-color 0.1s,
      color 0.1s;
  }

  .chip:hover {
    border-color: var(--accent-dim);
    color: var(--fg-primary);
  }

  .chip-name {
    color: inherit;
  }

  .chip-count {
    color: var(--fg-muted);
    font-size: 10px;
  }
</style>
