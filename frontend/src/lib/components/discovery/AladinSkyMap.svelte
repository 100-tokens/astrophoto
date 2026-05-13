<script lang="ts">
  /**
   * Embed of Aladin Lite v3 (CDS Strasbourg) — a WebGL sky-survey viewer
   * centered on this target's RA/Dec. The script is fetched lazily from the
   * CDS CDN on first mount; SSR is a no-op (server returns the placeholder).
   *
   * Skipped entirely when ra or dec is null — covers KEEP_MANUAL_META rows
   * (ic-434) and any custom seed without astro metadata (m40 etc.).
   *
   * Aladin Lite is GPL v3 / CDS-licensed. Attribution required (see footer).
   * Docs: https://aladin.cds.unistra.fr/AladinLite/doc/
   */
  interface Props {
    ra: number | null;
    dec: number | null;
    /** Major axis in arcminutes; drives the default FoV. Optional. */
    majorAxisArcmin?: number | null;
    /** Optional descriptive label for screen readers / aria. */
    objectName?: string;
  }

  let { ra, dec, majorAxisArcmin = null, objectName = 'celestial object' }: Props = $props();

  // CDS only publishes select versions under v3/<num>/ paths (3.2.0, 3.3.0,
  // and latest as of 2026-05). 'latest' is the supported stable channel; the
  // numbered paths skip many releases. Pin only if a regression breaks us.
  const ALADIN_SRC = 'https://aladin.cds.unistra.fr/AladinLite/api/v3/latest/aladin.js';

  let containerEl: HTMLDivElement | undefined = $state();
  let scriptLoaded = $state(false);
  let initialized = $state(false);

  /**
   * Sensible field-of-view: just over twice the object's major axis when known,
   * clamped to [0.1°, 5°]. For the unknown case, 0.5° catches most deep-sky
   * targets without losing the host field.
   */
  function computeFov(majorMinutes: number | null): number {
    if (majorMinutes === null || majorMinutes <= 0) return 0.5;
    const deg = (majorMinutes * 2.5) / 60;
    return Math.max(0.1, Math.min(5, deg));
  }

  $effect(() => {
    if (typeof window === 'undefined') return;
    if (ra === null || dec === null) return;

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const w = window as any;
    if (w.A) {
      scriptLoaded = true;
      return;
    }
    const existing = document.querySelector(`script[src="${ALADIN_SRC}"]`);
    if (existing) {
      existing.addEventListener('load', () => (scriptLoaded = true), { once: true });
      // Defensive: if it already loaded before we attached the listener.
      if (w.A) scriptLoaded = true;
      return;
    }
    const s = document.createElement('script');
    s.src = ALADIN_SRC;
    s.charset = 'utf-8';
    s.async = true;
    s.onload = () => (scriptLoaded = true);
    document.head.appendChild(s);
  });

  $effect(() => {
    if (!scriptLoaded || !containerEl || initialized) return;
    if (ra === null || dec === null) return;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const A = (window as any).A;
    if (!A?.init) return;

    initialized = true;
    A.init.then(() => {
      A.aladin(containerEl, {
        target: `${ra} ${dec}`,
        fov: computeFov(majorAxisArcmin),
        survey: 'P/DSS2/color',
        cooFrame: 'ICRSd',
        showReticle: false,
        showZoomControl: true,
        showLayersControl: true,
        showGotoControl: false,
        showShareControl: false,
        showFullscreenControl: true,
        showCooGridControl: false
      });
    });
  });
</script>

{#if ra !== null && dec !== null}
  <section class="sky-map" aria-label={`Sky map of ${objectName}`}>
    <header class="sky-map-header">
      <h2>Sky map</h2>
      <span class="t-meta">DSS2 colour · centred on RA/Dec</span>
    </header>
    <div bind:this={containerEl} class="aladin-container">
      {#if !scriptLoaded}
        <div class="loading">Loading sky map…</div>
      {/if}
    </div>
    <p class="attribution">
      Sky map by
      <a href="https://aladin.cds.unistra.fr/" target="_blank" rel="noopener noreferrer"
        >Aladin Lite</a
      >
      / CDS Strasbourg. Imagery from
      <a href="https://archive.eso.org/dss/dss" target="_blank" rel="noopener noreferrer">DSS2</a>
      (STScI, ESO, ROE, AAO).
    </p>
  </section>
{/if}

<style>
  .sky-map {
    margin: 1.5rem auto;
    max-width: 1100px;
    padding: 0 1rem;
  }
  .sky-map-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: 0.5rem;
  }
  .sky-map-header h2 {
    font-size: 1.05rem;
    font-weight: 600;
    margin: 0;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .aladin-container {
    width: 100%;
    height: 480px;
    background: var(--bg-faint, #0a0a0a);
    border: 1px solid var(--border-subtle, #2a2a2a);
    border-radius: var(--r-md, 6px);
    overflow: hidden;
    position: relative;
  }
  .loading {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--fg-muted, #888);
    font-size: 0.9rem;
  }
  .attribution {
    margin: 0.5rem 0 0;
    font-size: 0.75rem;
    color: var(--fg-muted, #888);
  }
  .attribution a {
    color: inherit;
    text-decoration: underline;
  }
</style>
