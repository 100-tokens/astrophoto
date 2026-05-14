<script lang="ts">
  import CategorySegmented from '$lib/components/CategorySegmented.svelte';
  import EquipmentAutocomplete from '$lib/components/EquipmentAutocomplete.svelte';
  import Img from '$lib/components/Img.svelte';
  import Input from '$lib/components/Input.svelte';
  import TagInput from '$lib/components/TagInput.svelte';
  import TargetPicker from '$lib/components/TargetPicker.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import { useAutosave } from '$lib/upload/useAutosave.svelte';
  import type { PhotoDetail } from '$lib/api/PhotoDetail';

  // The generated PhotoDetail type does not include the per-photo equipment
  // freetext fields (category, scope, mount, filters, guiding) — those are
  // written via the metadata patch but not echoed back in PhotoDetail yet.
  // Cast inline so we can seed the form from the server value.
  type ShowcasePhoto = PhotoDetail & {
    category?: string | null;
    scope?: string | null;
    mount?: string | null;
    filters?: string | null;
    guiding?: string | null;
  };

  interface Props {
    photo: PhotoDetail;
    initialTags: string[];
    autosave?: boolean;
  }

  let { photo, initialTags, autosave = false }: Props = $props();

  // Use a function to seed $state so ESLint/Svelte does not flag the prop
  // reference as a direct $state initializer dependency. These fields are
  // form-editable state, intentionally seeded once from the server value.
  function sp() {
    return photo as ShowcasePhoto;
  }
  const _sp = sp();
  function spTags() {
    return [...initialTags];
  }
  const _spTags = spTags();
  // Capture autosave at mount; parent remounts via {#key} when photo changes.
  function spAutosave() {
    return autosave;
  }
  const _autosave = spAutosave();

  let target = $state<string>(_sp.target ?? '');
  let category = $state<string>(_sp.category ?? 'other');
  let camera = $state<string>(_sp.camera ?? '');
  let scope = $state<string>(_sp.scope ?? '');
  let mount = $state<string>(_sp.mount ?? '');
  let filters = $state<string>(_sp.filters ?? '');
  let guiding = $state<string>(_sp.guiding ?? '');
  let tags = $state<string[]>(_spTags);
  let caption = $state<string>(_sp.caption ?? '');
  let lens = $state<string>(_sp.lens ?? '');
  let iso = $state<string>(_sp.iso?.toString() ?? '');
  let exposure_s = $state<string>(_sp.exposure_s?.toString() ?? '');
  let focal_mm = $state<string>(_sp.focal_mm?.toString() ?? '');
  let aperture_f = $state<string>(_sp.aperture_f?.toString() ?? '');
  let sessions = $state<string>(_sp.sessions?.toString() ?? '');
  let gain = $state<string>(_sp.gain?.toString() ?? '');
  let sensor_temp_c = $state<string>(_sp.sensor_temp_c?.toString() ?? '');
  let ra_deg = $state<string>(_sp.ra_deg?.toString() ?? '');
  let dec_deg = $state<string>(_sp.dec_deg?.toString() ?? '');

  let isProcessing = $derived(photo.status === 'processing');
  let isFailed = $derived(photo.status === 'failed');

  let recoveredCount = $derived.by(() => {
    let n = 0;
    if (photo.target) n++;
    if (photo.taken_at) n++;
    if (photo.camera) n++;
    if (photo.lens) n++;
    if (photo.iso != null) n++;
    if (photo.exposure_s != null) n++;
    if (photo.focal_mm != null) n++;
    if (photo.aperture_f != null) n++;
    if (photo.ra_deg != null && photo.dec_deg != null) n++;
    return n;
  });

  let snapshot = $derived(
    autosave
      ? {
          target: target || null,
          category,
          camera: camera || null,
          scope: scope || null,
          mount: mount || null,
          filters: filters || null,
          guiding: guiding || null,
          tags,
          caption: caption || null,
          lens: lens || null,
          iso: iso === '' ? null : Number(iso),
          exposure_s: exposure_s === '' ? null : Number(exposure_s),
          focal_mm: focal_mm === '' ? null : Number(focal_mm),
          aperture_f: aperture_f === '' ? null : Number(aperture_f),
          sessions: sessions === '' ? null : Number(sessions),
          gain: gain === '' ? null : Number(gain),
          sensor_temp_c: sensor_temp_c === '' ? null : Number(sensor_temp_c),
          ra_deg: ra_deg === '' ? null : Number(ra_deg),
          dec_deg: dec_deg === '' ? null : Number(dec_deg)
        }
      : null
  );

  const saver = _autosave ? useAutosave(_sp.id) : null;
  $effect(() => {
    return () => saver?.dispose();
  });
  $effect(() => {
    if (snapshot && saver) saver.queue(snapshot);
  });
</script>

<div class="layout">
  <aside class="preview" aria-label="Your upload">
    <div class="t-label">PREVIEW · CLIENT BITMAP</div>
    <div class="preview-frame">
      {#if isProcessing}
        <div class="processing-full">
          <div class="processing-eyebrow">● PROCESSING THUMBNAILS</div>
          <div class="processing-bar" aria-hidden="true"><span></span></div>
        </div>
      {:else if isFailed}
        <div class="failed-full">
          <div class="t-eyebrow danger">● PIPELINE FAILED</div>
          <p>{photo.pipeline_error ?? 'unknown error'}</p>
        </div>
      {:else}
        <Img
          photoId={photo.id}
          w={1200}
          alt={photo.target ?? photo.original_name}
          class="preview-img"
        />
        <div class="preview-overlay" aria-hidden="true">
          ● UPLOADED · DISPLAY MASTER {isProcessing ? 'QUEUED' : 'READY'}
        </div>
      {/if}
    </div>
    <div class="preview-caption t-meta">
      <span class="filename">{photo.original_name}</span>
      <span>
        {(Number(photo.bytes) / 1024 / 1024).toFixed(1)} MB
        {#if photo.width != null && photo.height != null}
          · {photo.width} × {photo.height}{/if}
      </span>
    </div>
  </aside>

  <div class="metadata">
    <div class="form-status">
      <span class="t-meta status-accent">
        ● {recoveredCount === 0
          ? 'NO EXIF FIELDS DETECTED'
          : `${recoveredCount} EXIF FIELD${recoveredCount === 1 ? '' : 'S'} RECOVERED`}
      </span>
      {#if autosave && saver}
        <span class="t-meta save-state" data-state={saver.state}>{saver.label}</span>
      {/if}
    </div>

    <fieldset disabled={isProcessing}>
      <!-- DISCOVERY ──────────────────────────────────────────────
           These four fields decide where this frame appears across
           the site (target page, category index, tag index). -->
      <section class="form-section section-accent">
        <header class="form-section-head">
          <span class="t-eyebrow accent">● DISCOVERY</span>
          <p class="t-meta section-help">
            These four fields decide where this frame appears across the site.
          </p>
        </header>
        <div class="field-full"><TargetPicker bind:value={target} /></div>
        <div class="field-full"><CategorySegmented bind:value={category} /></div>
        <div class="field-full"><TagInput bind:value={tags} /></div>
        <div class="field-full">
          <span class="t-label">CAPTION</span>
          <Textarea
            name="caption"
            rows={5}
            bind:value={caption}
            placeholder="Describe the conditions, processing, equipment used…"
          />
        </div>
      </section>

      <!-- EQUIPMENT USED ─────────────────────────────────────────── -->
      <section class="form-section">
        <header class="form-section-head">
          <span class="t-eyebrow">EQUIPMENT USED</span>
          <p class="t-meta section-help">
            Pre-fills from your default loadout. Each field becomes a browsable equipment page.
          </p>
        </header>
        <div class="grid equipment-grid">
          <div class="field">
            <EquipmentAutocomplete name="camera" kind="camera" bind:value={camera} />
          </div>
          <div class="field">
            <EquipmentAutocomplete name="scope" kind="telescope" bind:value={scope} />
          </div>
          <div class="field">
            <EquipmentAutocomplete name="mount" kind="mount" bind:value={mount} />
          </div>
          <div class="field">
            <EquipmentAutocomplete name="filters" kind="filter" bind:value={filters} />
          </div>
          <div class="field">
            <EquipmentAutocomplete name="guiding" kind="guiding" bind:value={guiding} />
          </div>
        </div>
      </section>

      <!-- CAPTURE DATA · FROM EXIF ──────────────────────────────── -->
      <section class="form-section">
        <header class="form-section-head">
          <span class="t-eyebrow">CAPTURE DATA · FROM EXIF</span>
          <p class="t-meta section-help">
            Editable, not required. We never overwrite the original file.
          </p>
        </header>
        <div class="grid">
          <label>
            <span class="t-label">LENS</span>
            <Input name="lens" bind:value={lens} />
          </label>
          <label>
            <span class="t-label">ISO</span>
            <Input type="number" name="iso" bind:value={iso} />
          </label>
          <label>
            <span class="t-label">EXPOSURE (S)</span>
            <Input type="number" step="0.01" name="exposure_s" bind:value={exposure_s} />
          </label>
          <label>
            <span class="t-label">FOCAL (MM)</span>
            <Input type="number" name="focal_mm" bind:value={focal_mm} />
          </label>
          <label>
            <span class="t-label">APERTURE (f/)</span>
            <Input type="number" step="0.1" name="aperture_f" bind:value={aperture_f} />
          </label>
          <label>
            <span class="t-label">SESSIONS</span>
            <Input type="number" name="sessions" bind:value={sessions} />
          </label>
          <label>
            <span class="t-label">GAIN</span>
            <Input type="number" name="gain" bind:value={gain} />
          </label>
          <label>
            <span class="t-label">SENSOR TEMP (°C)</span>
            <Input type="number" step="0.1" name="sensor_temp_c" bind:value={sensor_temp_c} />
          </label>
          <label>
            <span class="t-label">RA (DEG)</span>
            <Input type="number" step="0.0001" name="ra_deg" bind:value={ra_deg} />
          </label>
          <label>
            <span class="t-label">DEC (DEG)</span>
            <Input type="number" step="0.0001" name="dec_deg" bind:value={dec_deg} />
          </label>
        </div>
      </section>

      <!-- Plate-solve forward-compat banner ─────────────────────── -->
      <aside class="plate-solve" aria-label="Future feature: plate solving">
        <div>
          <span class="t-eyebrow accent">● COMING SOON · PLATE SOLVE</span>
          <p>
            When the astrometry phase ships, your frames here will be auto-matched to known targets.
            Targets you pick now stay as <code>source = "manual"</code>.
          </p>
        </div>
        <button type="button" class="plate-solve-btn" disabled>Run plate solve</button>
      </aside>
    </fieldset>
  </div>
</div>

<style>
  .layout {
    display: grid;
    grid-template-columns: 560px 1fr;
    gap: 64px;
    align-items: start;
  }
  .preview {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .preview-frame {
    position: relative;
    aspect-ratio: 4 / 3;
    background: var(--bg-elevated);
    overflow: hidden;
  }
  .preview :global(.preview-img) {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .processing-full,
  .failed-full {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 24px;
    background: rgba(12, 10, 8, 0.6);
    color: var(--fg-secondary);
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .processing-eyebrow {
    color: var(--accent);
    margin-bottom: 12px;
  }
  .processing-bar {
    width: 60%;
    height: 2px;
    background: var(--border-default);
    position: relative;
    overflow: hidden;
  }
  .processing-bar > span {
    position: absolute;
    inset: 0;
    width: 35%;
    background: var(--accent);
    animation: bar-slide 1.2s linear infinite;
  }
  @keyframes bar-slide {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(280%);
    }
  }
  .danger {
    color: var(--danger);
  }
  .form-status {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 16px;
    gap: 16px;
    flex-wrap: wrap;
  }
  .status-accent {
    color: var(--accent);
  }
  .save-state {
    font-family: var(--font-mono);
  }
  .save-state[data-state='error'] {
    color: var(--danger);
  }
  .field-full {
    margin-bottom: 16px;
  }
  /* ── Preview chrome ────────────────────────────────────────── */
  .preview-overlay {
    position: absolute;
    left: 12px;
    top: 12px;
    padding: 4px 8px;
    background: rgba(12, 10, 8, 0.85);
    border: 1px solid color-mix(in oklab, var(--accent) 50%, transparent);
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
  }
  .preview-caption {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    color: var(--fg-muted);
    /* Allow filename and size+dims to stack on narrow widths instead of
       hiding behind one another via space-between truncation. */
    flex-wrap: wrap;
  }
  .preview-caption .filename {
    font-family: var(--font-mono);
    color: var(--fg-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* ── Form sections ─────────────────────────────────────────── */
  .form-section {
    margin: 0 0 32px;
    border-top: 1px solid var(--border-subtle);
  }
  .form-section.section-accent {
    border-top-color: color-mix(in oklab, var(--accent) 60%, transparent);
  }
  .form-section-head {
    padding: 16px 0 4px;
  }
  .form-section-head .accent {
    color: var(--accent);
  }
  .section-help {
    margin: 6px 0 16px;
    color: var(--fg-muted);
    max-width: 56ch;
  }

  /* ── Plate-solve banner ────────────────────────────────────── */
  .plate-solve {
    margin: 24px 0 0;
    padding: 16px;
    border: 1px dashed color-mix(in oklab, var(--accent) 50%, transparent);
    background: color-mix(in oklab, var(--accent) 5%, transparent);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .plate-solve p {
    margin: 6px 0 0;
    color: var(--fg-secondary);
    font-size: 12px;
    line-height: 1.55;
    max-width: 60ch;
  }
  .plate-solve code {
    color: var(--accent);
    font-family: var(--font-mono);
  }
  .plate-solve-btn {
    background: transparent;
    border: 1px solid var(--border-default);
    color: var(--fg-muted);
    padding: 8px 14px;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.06em;
    opacity: 0.45;
    cursor: not-allowed;
    flex-shrink: 0;
  }

  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px 24px;
    margin-bottom: 16px;
  }
  .grid label {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .equipment-grid .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  @media (max-width: 1024px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }
  @media (max-width: 768px) {
    .grid {
      grid-template-columns: 1fr;
    }
    .form-section {
      margin-bottom: 24px;
    }
    .plate-solve {
      flex-direction: column;
      align-items: flex-start;
    }
  }
  @media (max-width: 480px) {
    .preview-overlay {
      font-size: 9px;
      padding: 3px 6px;
    }
  }
</style>
