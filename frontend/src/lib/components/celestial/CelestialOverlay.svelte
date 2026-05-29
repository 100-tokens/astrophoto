<script lang="ts">
  import type { CelestialObject } from '$lib/api/CelestialObject';
  import { projectRaDecToPixel, type Solve } from '$lib/utils/wcs';
  import { cssVarForType } from '$lib/utils/celestial-colors';

  let {
    objects,
    solve,
    layers,
    showPgc,
    labelsAlwaysOn,
    selectedSlug = $bindable(),
    onSelect
  }: {
    objects: CelestialObject[];
    solve: Solve;
    layers: Set<string>;
    showPgc: boolean;
    labelsAlwaysOn: boolean;
    selectedSlug: string | null;
    onSelect: (slug: string) => void;
  } = $props();

  // Project + filter once per (objects, solve, layers, showPgc) change.
  // Cap at 200 markers, ranked by descending size × confidence so the
  // biggest, most-confident objects always win the budget.
  let projected = $derived.by(() => {
    return objects
      .filter((o) => showPgc || o.kind !== 'pgc')
      .filter((o) => layers.has(o.objectType ?? 'other'))
      .map((o) => {
        const p = projectRaDecToPixel(o.rightAscension, o.declination, solve);
        if (!p) return null;
        // Marker radius tracks the object's true angular size, but is bounded
        // both ways: a 6px floor keeps tiny objects clickable, and a cap at
        // 45% of the frame's short edge keeps a frame-filling object (e.g. a
        // 28' nebula in a 23' field) as a contained circle rather than one
        // that spills far past the image edges.
        const maxR = 0.45 * Math.min(solve.width, solve.height);
        const trueR = ((o.majorAxisArcmin ?? 0.5) * 60) / solve.pixelScaleArcsec / 2;
        const radiusPx = Math.min(maxR, Math.max(6, trueR));
        return { o, x: p.x, y: p.y, r: radiusPx };
      })
      .filter(
        (x): x is { o: CelestialObject; x: number; y: number; r: number } => x !== null
      )
      .sort((a, b) => b.r * b.o.confidence - a.r * a.o.confidence)
      .slice(0, 200);
  });
</script>

<svg
  class="celestial-overlay"
  viewBox="0 0 {solve.width} {solve.height}"
  preserveAspectRatio="none"
  aria-label="Celestial-object overlay"
>
  {#each projected as { o, x, y, r } (o.slug)}
    {@const colorVar = cssVarForType(o.objectType)}
    <circle
      cx={x}
      cy={y}
      {r}
      fill="none"
      stroke="var({colorVar})"
      stroke-width={selectedSlug === o.slug ? 3 : 1.5}
      opacity={0.3 + 0.6 * o.confidence}
      class="marker"
      onclick={() => onSelect(o.slug)}
      onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && onSelect(o.slug)}
      role="button"
      tabindex="0"
      aria-label={o.canonicalName}
    >
      <title>{o.canonicalName} ({o.kind})</title>
    </circle>
    {#if labelsAlwaysOn || selectedSlug === o.slug}
      <text
        x={x + r + 4}
        y={y + 4}
        fill="var({colorVar})"
        font-size="14"
        font-family="ui-monospace, monospace"
        class="label"
      >
        {o.canonicalName}
      </text>
    {/if}
  {/each}
</svg>

<style>
  .celestial-overlay {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
  }
  .marker {
    pointer-events: auto;
    cursor: pointer;
  }
  .label {
    paint-order: stroke;
    stroke: #000;
    stroke-width: 3;
    pointer-events: none;
  }
</style>
