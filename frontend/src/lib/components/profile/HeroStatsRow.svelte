<script lang="ts">
  import type { HeroStats } from '$lib/api/HeroStats';
  import { formatIntegration } from '$lib/format/integration';

  let { stats }: { stats: HeroStats } = $props();

  // ts-rs maps Rust `i64` → TS `bigint`. Coerce for formatters that take number.
  let cells = $derived([
    { num: stats.frames.toLocaleString(), label: 'frames' },
    { num: formatIntegration(Number(stats.integration_seconds)), label: 'integration' },
    { num: stats.followers.toLocaleString(), label: 'followers' },
    { num: stats.appreciations.toLocaleString(), label: 'appreciations', accent: true },
    { num: stats.targets.toLocaleString(), label: 'targets shot' }
  ]);
</script>

<section class="row">
  {#each cells as c}
    <div class="cell" class:accent={c.accent}>
      <span class="num">{c.num}</span>
      <span class="lab">{c.label}</span>
    </div>
  {/each}
  <div class="member">Member since {stats.member_since_year}</div>
</section>

<style>
  .row {
    padding: 16px 32px;
    border-top: 1px solid var(--border-subtle);
    font-family: var(--font-mono);
    font-size: 12px;
    display: flex;
    flex-wrap: wrap;
    gap: 24px;
    align-items: baseline;
    color: var(--fg-secondary);
  }
  .cell {
    display: flex;
    gap: 6px;
    align-items: baseline;
  }
  .num {
    color: var(--fg-primary);
    font-size: 16px;
  }
  .lab {
    color: var(--fg-muted);
  }
  .accent .num {
    color: var(--accent);
  }
  .member {
    margin-left: auto;
    color: var(--fg-muted);
  }
</style>
