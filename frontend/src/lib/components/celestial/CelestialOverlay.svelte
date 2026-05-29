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
    zoomScale = 1,
    selectedSlug = $bindable(),
    onSelect
  }: {
    objects: CelestialObject[];
    solve: Solve;
    layers: Set<string>;
    showPgc: boolean;
    labelsAlwaysOn: boolean;
    /** Current image zoom factor. The SVG lives inside the zoom transform,
     *  so text is counter-scaled by 1/zoomScale to keep a constant on-screen
     *  size (markers still scale with the image — they show angular extent). */
    zoomScale?: number;
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

  // Label font size: a label is a UI annotation, not part of the sky, so it
  // should read at a constant on-screen size regardless of zoom. The SVG
  // sits inside the image's zoom transform (×zoomScale), so we divide the
  // base size by zoomScale — the transform then multiplies it back, netting
  // a constant ~13–15px on screen at any zoom level. The base (~2.2% of the
  // frame width) is what reads well at fit-to-width.
  let baseFontSize = $derived(Math.max(40, solve.width * 0.022));
  let fontSize = $derived(baseFontSize / zoomScale);

  // Vertical offset of the label below the marker, in viewBox units. Kept
  // independent of the (zoom-varying) font size so the label stays anchored
  // to the same point on the object as the user zooms; clamped so a
  // frame-filling circle's label sits just below centre, not off the edge.
  function labelY(y: number, r: number): number {
    const offset = Math.min(r, solve.height * 0.06) + solve.height * 0.022;
    return Math.min(solve.height - baseFontSize * 0.5, y + offset);
  }
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
      stroke-width={selectedSlug === o.slug ? 3 : 2}
      vector-effect="non-scaling-stroke"
      opacity={0.4 + 0.6 * o.confidence}
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
        x={x}
        y={labelY(y, r)}
        fill="var({colorVar})"
        font-size={fontSize}
        font-family="ui-monospace, monospace"
        text-anchor="middle"
        stroke="#000"
        stroke-width={fontSize * 0.18}
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
    pointer-events: none;
    font-weight: 600;
  }
</style>
