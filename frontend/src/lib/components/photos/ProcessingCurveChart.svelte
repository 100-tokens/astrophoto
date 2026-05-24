<script lang="ts">
  import type { ProcessTable } from '$lib/api/types';

  let { table }: { table: ProcessTable } = $props();

  const W = 168;
  const H = 120;
  const pad = 10;

  const points = $derived(
    table.rows
      .map((r) => ({ x: parseFloat(r[0] ?? ''), y: parseFloat(r[1] ?? '') }))
      .filter((p) => Number.isFinite(p.x) && Number.isFinite(p.y))
  );

  function sx(x: number): number {
    return pad + x * (W - 2 * pad);
  }
  function sy(y: number): number {
    return H - pad - y * (H - 2 * pad);
  }

  const path = $derived(
    points.map((p, i) => `${i === 0 ? 'M' : 'L'}${sx(p.x).toFixed(1)},${sy(p.y).toFixed(1)}`).join(' ')
  );
</script>

<svg viewBox="0 0 {W} {H}" class="curve" role="img" aria-label="Processing curve">
  <rect x={pad} y={pad} width={W - 2 * pad} height={H - 2 * pad} class="frame" />
  <line x1={pad} y1={H - pad} x2={W - pad} y2={pad} class="diagonal" />
  {#if path}
    <path d={path} class="curve-line" />
  {/if}
  {#each points as p (p.x + ':' + p.y)}
    <circle cx={sx(p.x)} cy={sy(p.y)} r="2.2" class="node" />
  {/each}
</svg>

<style>
  .curve {
    width: 168px;
    height: 120px;
    display: block;
  }
  .frame {
    fill: var(--bg-base);
    stroke: var(--border-subtle);
    stroke-width: 1;
  }
  .diagonal {
    stroke: var(--border-subtle);
    stroke-width: 1;
    stroke-dasharray: 3 3;
  }
  .curve-line {
    fill: none;
    stroke: var(--accent);
    stroke-width: 2;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
  .node {
    fill: var(--accent);
  }
</style>
