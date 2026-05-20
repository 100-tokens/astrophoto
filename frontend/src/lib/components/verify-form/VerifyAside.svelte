<script lang="ts">
  import Img from '$lib/components/Img.svelte';
  import type { XisfDisplayMeta } from '$lib/api/XisfDisplayMeta';

  // VerifyAside — sticky sidebar with frame preview, EXIF table,
  // conditional XISF processing history block, and Replace File link.
  //
  // Helpers `totalExposureLabel`, `observationSpan`, `fmtDateShort`,
  // `fmtCoord`, `xisfMetaIsEmpty` are duplicated from the legacy page
  // implementation verbatim (existing tests cover the formatting; we
  // don't want to break the contract).

  interface PhotoMin {
    id: string;
    target: string | null;
    original_name: string;
    width: number | null;
    height: number | null;
    bytes: number | bigint | string | null;
    camera: string | null;
    iso: number | null;
    exposure_s: number | null;
    gain: number | null;
    sensor_temp_c: number | null;
    sessions: number | null;
    taken_at?: string | null;
  }

  interface Props {
    photo: PhotoMin;
    xisfMeta: XisfDisplayMeta | null;
    isProcessing: boolean;
    isAwaitingCalibration: boolean;
    isPublished: boolean;
  }

  let {
    photo,
    xisfMeta = null,
    isProcessing = false,
    isAwaitingCalibration = false,
    isPublished = false
  }: Props = $props();

  let bytesLabel = $derived(
    Number(photo.bytes) ? `${(Number(photo.bytes) / (1024 * 1024)).toFixed(1)} MB` : null
  );
  let dimensionLabel = $derived(
    photo.width && photo.height ? `${photo.width} × ${photo.height}` : null
  );
  let metaLine = $derived([bytesLabel, dimensionLabel].filter((x) => x !== null).join(' · '));

  function totalExposureLabel(totalS: number, perSubS: number | null | undefined): string {
    const round = (x: number) => Math.round(x);
    const parts: string[] = [`${round(totalS)} s`];
    const hours = Math.floor(totalS / 3600);
    const mins = Math.round((totalS - hours * 3600) / 60);
    parts.push(`= ${hours} h ${mins.toString().padStart(2, '0')} min`);
    if (perSubS && perSubS > 0) {
      const subs = totalS / perSubS;
      const subsLabel = Number.isInteger(subs) ? `${subs}` : `~${Math.round(subs)}`;
      parts.push(`· ${subsLabel} subs of ${round(perSubS)} s`);
    }
    return parts.join(' ');
  }

  function observationSpan(startIso: string, endIso: string): string {
    const start = new Date(startIso);
    const end = new Date(endIso);
    const ms = end.getTime() - start.getTime();
    if (!Number.isFinite(ms) || ms < 0) return '';
    const hours = ms / (1000 * 60 * 60);
    if (hours < 24) return `${hours.toFixed(1)} h`;
    const days = Math.floor(hours / 24);
    const remHours = Math.round(hours - days * 24);
    return `${days} d ${remHours.toString().padStart(2, '0')} h`;
  }

  function fmtDateShort(iso: string): string {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return iso;
    return d.toISOString().slice(0, 10);
  }

  function fmtCoord(deg: number, suffix: [string, string]): string {
    const sign = deg >= 0 ? suffix[0] : suffix[1];
    return `${Math.abs(deg).toFixed(4)}° ${sign}`;
  }

  function xisfMetaIsEmpty(m: XisfDisplayMeta): boolean {
    return (
      !m.filter &&
      !m.telescope &&
      !m.observationStart &&
      !m.observationEnd &&
      m.latitudeDeg == null &&
      m.longitudeDeg == null &&
      m.elevationM == null &&
      m.subframes == null &&
      m.binningX == null &&
      m.binningY == null &&
      m.history.length === 0 &&
      m.totalExposureS == null
    );
  }

  let hasXisf = $derived(!!xisfMeta && !xisfMetaIsEmpty(xisfMeta));
</script>

<aside class="aside" aria-label="Your upload">
  <div class="t-label aside-head">YOUR UPLOAD</div>

  <div class="frame corner-marks">
    <div class="frame-inner">
      {#if isProcessing}
        <div class="frame-overlay">
          <div class="t-eyebrow accent">
            ● {isAwaitingCalibration ? 'PLATE-SOLVING XISF' : 'PROCESSING THUMBNAILS'}
          </div>
          <div class="frame-bar" aria-hidden="true"><span></span></div>
        </div>
      {/if}
      <Img
        photoId={photo.id}
        w={1200}
        alt={photo.target ?? photo.original_name}
        class="frame-img"
      />
    </div>
  </div>

  <div class="filename-row">
    <span class="filename">{photo.original_name}</span>
    <span class="t-meta">{metaLine || ''}</span>
  </div>

  <div class="exif-block">
    <div class="t-label aside-subhead">READ FROM FILE</div>
    <table class="exif">
      <tbody>
        <tr>
          <th>Camera</th>
          <td>{photo.camera ?? '—'}</td>
        </tr>
        <tr>
          <th>Sub exposure</th>
          <td>{photo.exposure_s != null ? `${photo.exposure_s} s` : '—'}</td>
        </tr>
        <tr>
          <th>ISO / Gain</th>
          <td>{photo.iso ?? '—'} / {photo.gain ?? '—'}</td>
        </tr>
        <tr>
          <th>Sensor temp</th>
          <td>{photo.sensor_temp_c != null ? `${photo.sensor_temp_c} °C` : '—'}</td>
        </tr>
        <tr>
          <th>Frames captured</th>
          <td>{photo.sessions ?? '—'}</td>
        </tr>
        {#if xisfMeta?.filter}
          <tr><th>Filter</th><td>{xisfMeta.filter}</td></tr>
        {/if}
        {#if xisfMeta?.telescope}
          <tr><th>Telescope</th><td>{xisfMeta.telescope}</td></tr>
        {/if}
        {#if xisfMeta?.binningX != null && xisfMeta?.binningY != null}
          <tr><th>Binning</th><td>{xisfMeta.binningX}×{xisfMeta.binningY}</td></tr>
        {/if}
        {#if xisfMeta?.observationStart}
          <tr>
            <th>Captured</th>
            <td>
              {#if xisfMeta.observationEnd && xisfMeta.observationEnd !== xisfMeta.observationStart}
                {fmtDateShort(xisfMeta.observationStart)} → {fmtDateShort(xisfMeta.observationEnd)}
                ({observationSpan(xisfMeta.observationStart, xisfMeta.observationEnd)})
              {:else}
                {fmtDateShort(xisfMeta.observationStart)}
              {/if}
            </td>
          </tr>
        {/if}
        {#if xisfMeta?.totalExposureS != null}
          <tr>
            <th>Integration</th>
            <td>{totalExposureLabel(xisfMeta.totalExposureS, photo.exposure_s)}</td>
          </tr>
        {/if}
        {#if xisfMeta?.latitudeDeg != null && xisfMeta?.longitudeDeg != null}
          <tr>
            <th>Site GPS</th>
            <td>
              {fmtCoord(xisfMeta.latitudeDeg, ['N', 'S'])} · {fmtCoord(xisfMeta.longitudeDeg, [
                'E',
                'W'
              ])}{#if xisfMeta.elevationM != null}
                · {Math.round(xisfMeta.elevationM)} m{/if}
            </td>
          </tr>
        {/if}
      </tbody>
    </table>
  </div>

  {#if hasXisf && xisfMeta && xisfMeta.history.length > 0}
    <div class="history-block">
      <div class="t-label aside-subhead">PROCESSING HISTORY</div>
      <ol class="history-list">
        {#each xisfMeta.history as line, i (i)}
          <li class="history-row">
            <span class="history-idx">{String(i + 1).padStart(2, '0')}</span>
            <span class="history-text">{line}</span>
          </li>
        {/each}
      </ol>
    </div>
  {/if}

  {#if !isPublished}
    <div class="replace-row">
      <a class="replace-link" href="/upload" data-sveltekit-reload>← REPLACE FILE</a>
    </div>
  {/if}
</aside>

<style>
  .aside {
    position: sticky;
    top: 24px;
    display: block;
  }
  .aside-head {
    margin-bottom: 16px;
  }
  .aside-subhead {
    margin-bottom: 10px;
  }
  .frame {
    position: relative;
    padding: 10px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-base);
  }
  .frame-inner {
    position: relative;
    aspect-ratio: 3 / 2;
    width: 100%;
    background: var(--bg-elevated);
    overflow: hidden;
  }
  .frame :global(.frame-img) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .frame-overlay {
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: 12px;
    z-index: 1;
    padding: 8px 12px;
    background: rgba(12, 10, 8, 0.85);
    border: 1px solid var(--border-default);
  }
  .frame-bar {
    margin-top: 6px;
    height: 2px;
    background: var(--border-default);
    position: relative;
    overflow: hidden;
  }
  .frame-bar > span {
    position: absolute;
    inset: 0;
    width: 35%;
    background: var(--accent);
    animation: aside-bar-slide 1.2s linear infinite;
  }
  @keyframes aside-bar-slide {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(280%);
    }
  }
  .filename-row {
    margin-top: 14px;
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
    padding-bottom: 14px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .filename {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 60%;
  }
  .exif-block {
    margin-top: 20px;
  }
  .history-block {
    margin-top: 28px;
  }
  .history-list {
    list-style: none;
    padding: 0;
    margin: 0;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-secondary);
  }
  .history-row {
    display: grid;
    grid-template-columns: 24px 1fr;
    gap: 10px;
    padding: 8px 0;
    border-bottom: 1px dashed var(--border-subtle);
  }
  .history-row:last-child {
    border-bottom: 0;
  }
  .history-idx {
    color: var(--fg-faint);
  }
  .history-text {
    color: var(--fg-secondary);
    word-break: break-word;
  }
  .replace-row {
    margin-top: 32px;
    padding-top: 20px;
    border-top: 1px solid var(--border-subtle);
  }
  .replace-link {
    font-family: var(--font-mono);
    font-size: 12px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-muted);
    cursor: pointer;
  }
  .replace-link:hover {
    color: var(--accent);
  }
  @media (max-width: 1023px) {
    .aside {
      position: static;
    }
  }
</style>
