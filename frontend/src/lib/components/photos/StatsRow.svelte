<script lang="ts">
  let {
    stats
  }: {
    stats: {
      published_count: number;
      draft_count: number;
      integration_secs: number;
      appreciations_received: number;
    };
  } = $props();

  function formatHours(secs: number): string {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    return `${h} h ${m} m`;
  }
</script>

<div class="stats-row">
  <div class="cell">
    <span class="t-label">PUBLISHED</span><span class="num">{stats.published_count}</span>
  </div>
  <div class="cell" class:accent={stats.draft_count > 0}>
    <span class="t-label">DRAFTS</span><span class="num">{stats.draft_count}</span>
  </div>
  <div class="cell">
    <span class="t-label">TOTAL INTEGRATION</span><span class="num"
      >{formatHours(stats.integration_secs)}</span
    >
  </div>
  <div class="cell">
    <span class="t-label">APPRECIATIONS</span><span class="num">{stats.appreciations_received}</span
    >
  </div>
</div>

<style>
  .stats-row {
    display: flex;
    gap: 32px;
  }
  .cell {
    display: flex;
    flex-direction: column;
    gap: 4px;
    align-items: flex-end;
  }
  .num {
    font-family: var(--font-display);
    font-size: 24px;
  }
  .cell.accent .num {
    color: var(--accent);
  }
</style>
