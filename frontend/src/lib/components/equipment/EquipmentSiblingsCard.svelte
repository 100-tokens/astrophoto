<script lang="ts">
  import type { EquipmentSibling } from '$lib/api/EquipmentSibling';

  interface Props {
    siblings: EquipmentSibling[];
    brand: string;
  }

  let { siblings, brand }: Props = $props();

  function fmt(n: number): string {
    return Number(n).toLocaleString('en-US');
  }
</script>

{#if siblings.length > 0}
  <aside class="sib-card">
    <div class="t-label">OTHER {brand.toUpperCase()}</div>
    <ul>
      {#each siblings as s (s.kind + '/' + s.slug)}
        <li>
          <a href="/equip/{s.kind}/{s.slug}">
            <span class="name">{s.display_name}</span>
            <span class="count">{fmt(s.usage_count)}</span>
          </a>
        </li>
      {/each}
    </ul>
  </aside>
{/if}

<style>
  .sib-card {
    padding: 20px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-raised);
  }
  ul {
    list-style: none;
    margin: 12px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  li a {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
    padding: 6px 0;
    text-decoration: none;
    color: var(--fg-secondary);
    border-bottom: 1px solid var(--border-subtle);
    transition: color 0.12s;
  }
  li:last-child a {
    border-bottom: none;
  }
  li a:hover {
    color: var(--fg-primary);
  }
  .name {
    font-family: var(--font-display);
    font-style: italic;
    font-size: 14px;
    flex: 1;
    min-width: 0;
  }
  .count {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
  }
</style>
