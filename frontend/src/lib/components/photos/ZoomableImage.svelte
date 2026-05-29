<script lang="ts">
  // Pan + zoom viewer for the photo-page main image. Wheel zooms toward
  // the cursor; drag pans (bounded so the frame never shows gaps);
  // double-click toggles zoom; controls + reset. Astrophotos reward
  // close inspection — this is the core of that experience.
  import { cdn } from '$lib/cdn';
  import type { Snippet } from 'svelte';

  // `maxHeight` caps the image so it fits the viewport; pass a context-
  // specific value (photo page vs fullscreen lightbox). Self-contained:
  // the component sizes its own image, no parent CSS required.
  //
  // `overlay` is an optional snippet rendered ON TOP of the image, inside
  // the same pan/zoom transform layer — so an annotation overlay (e.g. the
  // celestial-object markers) tracks the image exactly as the user zooms
  // and pans. It is sized to the image's displayed box, so an absolutely-
  // positioned child filling `inset:0` covers exactly the pixels of the photo.
  // The snippet receives the current zoom `scale` so it can counter-scale
  // any element that should stay a constant on-screen size (e.g. text labels)
  // while everything else scales with the image.
  let {
    photoId,
    alt,
    w = 2560,
    maxHeight = '80vh',
    overlay
  }: {
    photoId: string;
    alt: string;
    w?: number;
    maxHeight?: string;
    overlay?: Snippet<[number]>;
  } = $props();

  let viewer: HTMLDivElement | undefined = $state();
  let img: HTMLImageElement | undefined = $state();

  let scale = $state(1);
  let tx = $state(0);
  let ty = $state(0);
  const MIN = 1;
  const MAX = 6;

  let dragging = $state(false);
  let moved = false;
  let lastX = 0;
  let lastY = 0;

  const zoomed = $derived(scale > 1.001);

  // Max pan offset at current scale, so the contain-fitted image edges
  // never pull inside the frame (no gaps). Uses the pre-transform size.
  function maxOffset(): { x: number; y: number } {
    if (!viewer || !img) return { x: 0, y: 0 };
    const vw = viewer.clientWidth;
    const vh = viewer.clientHeight;
    const bw = img.offsetWidth * scale;
    const bh = img.offsetHeight * scale;
    return { x: Math.max(0, (bw - vw) / 2), y: Math.max(0, (bh - vh) / 2) };
  }

  function clamp() {
    const m = maxOffset();
    tx = Math.max(-m.x, Math.min(m.x, tx));
    ty = Math.max(-m.y, Math.min(m.y, ty));
    if (scale <= MIN) {
      tx = 0;
      ty = 0;
    }
  }

  function zoomAt(clientX: number, clientY: number, next: number) {
    if (!viewer) return;
    const r = viewer.getBoundingClientRect();
    const cx = clientX - r.left - r.width / 2; // cursor rel. to center
    const cy = clientY - r.top - r.height / 2;
    const ns = Math.max(MIN, Math.min(MAX, next));
    // keep the image point under the cursor fixed (origin = center)
    tx = cx - ((cx - tx) * ns) / scale;
    ty = cy - ((cy - ty) * ns) / scale;
    scale = ns;
    clamp();
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();
    const factor = Math.exp(-e.deltaY * 0.0015);
    zoomAt(e.clientX, e.clientY, scale * factor);
  }

  function onPointerDown(e: PointerEvent) {
    if (!zoomed) return;
    dragging = true;
    moved = false;
    lastX = e.clientX;
    lastY = e.clientY;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function onPointerMove(e: PointerEvent) {
    if (!dragging) return;
    const dx = e.clientX - lastX;
    const dy = e.clientY - lastY;
    if (Math.abs(dx) + Math.abs(dy) > 2) moved = true;
    tx += dx;
    ty += dy;
    lastX = e.clientX;
    lastY = e.clientY;
    clamp();
  }
  function onPointerUp(e: PointerEvent) {
    dragging = false;
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      /* pointer already released */
    }
  }

  function onDblClick(e: MouseEvent) {
    if (zoomed) reset();
    else zoomAt(e.clientX, e.clientY, 2.6);
  }

  function reset() {
    scale = 1;
    tx = 0;
    ty = 0;
  }
  function zoomBy(f: number) {
    if (!viewer) return;
    const r = viewer.getBoundingClientRect();
    zoomAt(r.left + r.width / 2, r.top + r.height / 2, scale * f);
  }
</script>

<div
  class="viewer"
  class:zoomed
  class:dragging
  bind:this={viewer}
  style="--zi-max-h: {maxHeight}"
  onwheel={onWheel}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerUp}
  ondblclick={onDblClick}
  role="presentation"
>
  <div
    class="tlayer"
    style="transform: translate({tx}px, {ty}px) scale({scale});"
  >
    <img bind:this={img} src={cdn(photoId, { w })} {alt} draggable="false" />
    {#if overlay}
      <div class="overlay-slot">{@render overlay(scale)}</div>
    {/if}
  </div>

  <div class="controls" class:hidden={!zoomed && false}>
    <button type="button" aria-label="Zoom in" onclick={() => zoomBy(1.5)}>+</button>
    <button type="button" aria-label="Zoom out" onclick={() => zoomBy(1 / 1.5)}>−</button>
    <button type="button" aria-label="Reset zoom" onclick={reset} disabled={!zoomed}>⤢</button>
  </div>
  {#if !zoomed}
    <span class="hint" aria-hidden="true">Scroll or double-click to zoom</span>
  {/if}
</div>

<style>
  .viewer {
    position: relative;
    width: 100%;
    overflow: hidden;
    touch-action: none;
    cursor: zoom-in;
    background: var(--bg-base);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .viewer.zoomed {
    cursor: grab;
  }
  .viewer.zoomed.dragging {
    cursor: grabbing;
  }
  /* Self-sizing: the image is capped by max-height (per-context via the
     --zi-max-h prop) and the viewer's width; the frame follows it, and
     overflow:hidden clips it when zoomed. Works on the photo page and in
     the fullscreen lightbox with no parent CSS. */
  /* The transform layer shrink-wraps the image and carries the pan/zoom
     transform, so any overlay snippet inside it tracks the photo exactly. */
  .tlayer {
    position: relative;
    display: inline-block;
    max-width: 100%;
    max-height: var(--zi-max-h, 80vh);
    line-height: 0;
    transform-origin: center center;
    will-change: transform;
  }
  .viewer:not(.dragging) .tlayer {
    transition: transform 120ms var(--ease-out, ease-out);
  }
  img {
    display: block;
    max-width: 100%;
    max-height: var(--zi-max-h, 80vh);
    width: auto;
    height: auto;
    object-fit: contain;
    user-select: none;
    -webkit-user-drag: none;
  }
  /* Covers exactly the image's displayed box; children (the SVG overlay)
     fill inset:0 and inherit the transform from .tlayer. */
  .overlay-slot {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .overlay-slot :global(svg),
  .overlay-slot :global(.celestial-overlay) {
    pointer-events: none;
  }
  .overlay-slot :global(.marker) {
    pointer-events: auto;
  }
  .controls {
    position: absolute;
    bottom: 12px;
    right: 12px;
    display: flex;
    gap: 6px;
    opacity: 0;
    transition: opacity 150ms var(--ease-out, ease-out);
  }
  .viewer:hover .controls,
  .viewer.zoomed .controls {
    opacity: 1;
  }
  .controls button {
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    background: color-mix(in oklab, var(--bg-base) 75%, transparent);
    backdrop-filter: blur(6px);
    border: 1px solid var(--border-default);
    border-radius: var(--r-md);
    color: var(--fg-primary);
    font-size: 15px;
    line-height: 1;
    cursor: pointer;
  }
  .controls button:hover {
    border-color: var(--accent);
  }
  .controls button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .hint {
    position: absolute;
    bottom: 12px;
    left: 12px;
    font-size: 0.72rem;
    color: var(--fg-faint);
    font-family: var(--font-mono);
    opacity: 0;
    transition: opacity 150ms var(--ease-out, ease-out);
    pointer-events: none;
  }
  .viewer:hover .hint {
    opacity: 1;
  }
</style>
