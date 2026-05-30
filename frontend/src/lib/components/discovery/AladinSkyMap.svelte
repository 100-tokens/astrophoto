<script lang="ts">
  /**
   * Embed of Aladin Lite v3 (CDS Strasbourg) — a WebGL sky-survey viewer
   * centered on this target's RA/Dec. To keep the page fast we show a single
   * static DSS2 preview image (CDS hips2fits) by default and only fetch the
   * heavy Aladin script + tiles when the user clicks to open the interactive
   * viewer; SSR renders just the preview.
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
  // The interactive Aladin viewer is loaded only when the user opens it; until
  // then we show a single static DSS2 preview image (see previewUrl).
  let interactive = $state(false);
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

  // Static DSS2 preview: one server-rendered JPEG from the CDS hips2fits
  // service, shown until the user opens the interactive viewer. The heavy
  // Aladin Lite library + tiles (~4-5s of CDS requests) then load only on
  // click, so the default page stays fast. (The map sits above the fold, so a
  // scroll-lazy load could not defer it — hence click-to-open.)
  const previewUrl = $derived(
    ra === null || dec === null
      ? ''
      : 'https://alasky.cds.unistra.fr/hips-image-services/hips2fits?hips=CDS/P/DSS2/color' +
          `&width=1000&height=480&fov=${computeFov(majorAxisArcmin)}` +
          `&projection=TAN&coordsys=icrs&ra=${ra}&dec=${dec}&format=jpg`
  );

  // Inject the Aladin script from the CDS CDN — only once the user opens the
  // interactive map, so the default page load stays fast.
  $effect(() => {
    if (typeof window === 'undefined' || !interactive) return;
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
      {#if !interactive}
        <button
          type="button"
          class="map-preview"
          onclick={() => (interactive = true)}
          aria-label={`Open the interactive sky map of ${objectName}`}
        >
          {#if previewUrl}
            <img
              class="map-preview-img"
              src={previewUrl}
              alt={`DSS2 sky-survey image centred on ${objectName}`}
              loading="lazy"
              width="1000"
              height="480"
            />
          {/if}
          <span class="map-preview-cta">▶ Open interactive map</span>
        </button>
      {:else if !scriptLoaded}
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
    background: var(--bg-canvas);
    border: 1px solid var(--border-subtle, #2a2a2a);
    border-radius: var(--r-md);
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
  .map-preview {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    padding: 0;
    border: 0;
    background: var(--bg-canvas);
    cursor: pointer;
    display: block;
    overflow: hidden;
  }
  .map-preview-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .map-preview-cta {
    position: absolute;
    left: 50%;
    bottom: 16px;
    transform: translateX(-50%);
    background: rgba(0, 0, 0, 0.72);
    color: #fff;
    padding: 8px 16px;
    border-radius: var(--r-md);
    font-family: var(--font-mono);
    font-size: 0.85rem;
    pointer-events: none;
  }
  .map-preview:hover .map-preview-cta,
  .map-preview:focus-visible .map-preview-cta {
    background: var(--accent, #4a90e2);
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
