<script lang="ts">
  import type { CelestialObject } from '$lib/api/CelestialObject';
  import type { Component } from 'svelte';

  let { object, onBack }: { object: CelestialObject; onBack: () => void } = $props();

  // Lazy-load AladinSkyMap only when this component mounts (first time
  // the user opens any detail). The dynamic import is cached by Vite
  // after the first call, so subsequent opens are instant.
  let AladinComp = $state<Component<{
    ra: number;
    dec: number;
    majorAxisArcmin?: number | null;
    objectName?: string;
  }> | null>(null);

  $effect(() => {
    void import('$lib/components/discovery/AladinSkyMap.svelte')
      .then((m) => (AladinComp = m.default as typeof AladinComp))
      .catch((e) => console.warn('Aladin lazy-load failed', e));
  });
</script>

<div class="celestial-detail">
  <header>
    <button type="button" onclick={onBack} aria-label="back to list">← back</button>
    <h4>{object.canonicalName}</h4>
  </header>
  <p class="meta">
    {object.objectType ?? '—'}
    {#if object.magnitudeV != null}
      · mag {object.magnitudeV.toFixed(1)}{/if}
    {#if object.majorAxisArcmin != null}
      · {object.majorAxisArcmin.toFixed(1)}′{#if object.minorAxisArcmin != null}
        × {object.minorAxisArcmin.toFixed(1)}′{/if}
    {/if}
  </p>
  <p class="coord">
    RA {object.rightAscension.toFixed(4)}° · Dec {object.declination.toFixed(4)}°
  </p>

  <div class="aladin-wrap">
    {#if AladinComp}
      <AladinComp
        ra={object.rightAscension}
        dec={object.declination}
        majorAxisArcmin={object.majorAxisArcmin}
        objectName={object.canonicalName}
      />
    {:else}
      <div class="placeholder">loading sky map…</div>
    {/if}
  </div>

  <a class="full-link" href="/t/{object.slug}">→ /t/{object.slug}</a>
</div>

<style>
  .celestial-detail header {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .celestial-detail button {
    background: none;
    border: 0;
    color: var(--accent);
    cursor: pointer;
    font-size: 11px;
    font-family: ui-monospace, monospace;
  }
  .celestial-detail h4 {
    margin: 0;
    flex: 1;
    font-size: 12px;
  }
  .celestial-detail .meta,
  .celestial-detail .coord {
    font-size: 12px;
    color: var(--fg-muted);
    margin: 4px 0;
  }
  .aladin-wrap {
    margin-top: 8px;
    border: 1px solid var(--border-default);
    border-radius: var(--r-md);
    aspect-ratio: 1 / 1;
    overflow: hidden;
  }
  .placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--fg-muted);
    font-size: 11px;
  }
  .full-link {
    display: block;
    margin-top: 8px;
    color: var(--accent);
    font-size: 12px;
  }
</style>
