<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import Button from '$lib/components/Button.svelte';
  import CategorySegmented from '$lib/components/CategorySegmented.svelte';
  import EquipmentAutocomplete from '$lib/components/EquipmentAutocomplete.svelte';
  import FilterChipInput from '$lib/components/equipment/FilterChipInput.svelte';
  import Img from '$lib/components/Img.svelte';
  import Input from '$lib/components/Input.svelte';
  import SetupPicker from '$lib/components/SetupPicker.svelte';
  import TagInput from '$lib/components/TagInput.svelte';
  import TargetPicker from '$lib/components/TargetPicker.svelte';
  import UploadStepper from '$lib/components/UploadStepper.svelte';
  import type { PhotoFilterChip } from '$lib/api/PhotoFilterChip';
  import type { PlatesolveStatus } from '$lib/api/PlatesolveStatus';
  import type { SetupSummary } from '$lib/api/SetupSummary';
  import type { PageProps } from './$types';

  const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '';
  // Mirrors backend/src/photos/platesolve.rs::MAX_XISF_BYTES. Kept as a
  // literal here so the client surfaces a friendly "too large" error
  // before uploading a body the server would reject with 413.
  const PLATESOLVE_MAX_BYTES = 128 * 1024 * 1024;
  const PLATESOLVE_POLL_INTERVAL_MS = 2000;

  let { data, form }: PageProps = $props();
  // Silence unused-import warning: SetupSummary is used only for the type cast below.
  type _SetupSummary = SetupSummary;
  let polling = $state<number | null>(null);

  // Plate-solve panel state. Initial status is seeded by the server
  // load so the panel renders the correct UI on the first paint;
  // subsequent updates come from the polling loop below. The data
  // read is wrapped in a function to dodge the "only captures the
  // initial value" lint — initial seeding is intentional here, the
  // polling loop owns updates thereafter.
  function initialPlatesolveStatus(): PlatesolveStatus | null {
    return data.platesolveStatus ?? null;
  }
  let psStatus = $state<PlatesolveStatus | null>(initialPlatesolveStatus());
  let psFile = $state<File | null>(null);
  let psSubmitting = $state(false);
  let psLocalError = $state<string | null>(null);
  let psPollHandle = $state<number | null>(null);
  let psState = $derived(psStatus?.state ?? 'idle');
  let psIsSolving = $derived(psState === 'solving');

  function onPickXisf(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0] ?? null;
    psLocalError = null;
    if (!file) {
      psFile = null;
      return;
    }
    if (file.size > PLATESOLVE_MAX_BYTES) {
      psLocalError = `File is ${(file.size / (1024 * 1024)).toFixed(0)} MB — the plate-solve service accepts up to ${PLATESOLVE_MAX_BYTES / (1024 * 1024)} MB.`;
      psFile = null;
      input.value = '';
      return;
    }
    psFile = file;
  }

  async function onCalibrate() {
    if (!psFile || psSubmitting) return;
    psSubmitting = true;
    psLocalError = null;
    try {
      const body = new FormData();
      // The field name matches the backend handler's `match field.name() { Some("file") => ... }`.
      body.append('file', psFile);
      const r = await fetch(`${API}/api/photos/${data.photo.id}/platesolve`, {
        method: 'POST',
        credentials: 'include',
        body
      });
      if (r.status === 202 || r.status === 409) {
        // 202 = our solve just kicked off. 409 = a concurrent solve
        // (e.g. another tab) is already in flight — either way we
        // join the same in-progress state and start polling.
        psStatus = { ...(psStatus ?? emptyStatus()), state: 'solving', error: null };
        startPolling();
      } else {
        const text = await r.text();
        psLocalError = `Upload failed (${r.status}): ${text.slice(0, 200)}`;
      }
    } catch (e) {
      psLocalError = `Network error: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      psSubmitting = false;
    }
  }

  function emptyStatus(): PlatesolveStatus {
    return {
      state: 'idle',
      error: null,
      solvedAt: null,
      raDeg: null,
      decDeg: null,
      pixelScaleArcsec: null,
      rotationDeg: null,
      rmsArcsec: null,
      matchedCount: null,
      detectedCount: null
    };
  }

  async function fetchPlatesolveStatus(): Promise<PlatesolveStatus | null> {
    try {
      const r = await fetch(`${API}/api/photos/${data.photo.id}/platesolve-status`, {
        credentials: 'include'
      });
      if (!r.ok) return null;
      return (await r.json()) as PlatesolveStatus;
    } catch {
      return null;
    }
  }

  function startPolling() {
    if (psPollHandle !== null) return;
    psPollHandle = window.setInterval(async () => {
      const next = await fetchPlatesolveStatus();
      if (!next) return;
      const wasSolving = psStatus?.state === 'solving';
      psStatus = next;
      if (next.state !== 'solving') {
        stopPolling();
        if (wasSolving && next.state === 'solved') {
          // Refresh PhotoDetail so ra_deg/dec_deg inputs pick up the
          // server-side value. invalidateAll() re-runs the load.
          void invalidateAll();
        }
      }
    }, PLATESOLVE_POLL_INTERVAL_MS);
  }

  function stopPolling() {
    if (psPollHandle !== null) {
      clearInterval(psPollHandle);
      psPollHandle = null;
    }
  }

  // Seed polling on mount if the server says we're already mid-solve
  // (e.g. user navigated away and came back).
  $effect(() => {
    if (psIsSolving) startPolling();
    else stopPolling();
    return () => stopPolling();
  });

  function formatTelemetry(s: PlatesolveStatus): string {
    const parts: string[] = [];
    if (s.raDeg != null && s.decDeg != null) {
      parts.push(`RA ${s.raDeg.toFixed(4)}° · Dec ${s.decDeg.toFixed(4)}°`);
    }
    if (s.pixelScaleArcsec != null) parts.push(`${s.pixelScaleArcsec.toFixed(3)}″/px`);
    if (s.rotationDeg != null) parts.push(`rot ${s.rotationDeg.toFixed(2)}°`);
    if (s.rmsArcsec != null) parts.push(`RMS ${s.rmsArcsec.toFixed(3)}″`);
    if (s.matchedCount != null && s.detectedCount != null) {
      parts.push(`matched ${s.matchedCount}/${s.detectedCount}`);
    }
    return parts.join(' · ');
  }

  // The generated PhotoDetail type still doesn't include the per-photo
  // equipment freetext fields (category, scope, mount, guiding) —
  // those are written via the metadata patch but not echoed back in
  // PhotoDetail. Cast inline so we can seed the form from the server value.
  type ShowcasePhoto = typeof data.photo & {
    setup_id?: string | null;
    category?: string | null;
    scope?: string | null;
    focal_modifier?: string | null;
    mount?: string | null;
    filters?: string | null;
    guiding?: string | null;
  };
  // Cast data.photo inside a function so ESLint does not see the prop reference
  // as a direct $state initializer dependency (these fields are form-editable
  // state, intentionally seeded once from the server value).
  function initialPhoto() {
    return data.photo as ShowcasePhoto;
  }
  const _sp = initialPhoto();
  function initialFilterChips() {
    return (data.photo.filter_items ?? []) as PhotoFilterChip[];
  }
  const _filterChips = initialFilterChips();

  let target = $state<string>(_sp.target ?? '');
  let camera = $state<string>(_sp.camera ?? '');
  let tags = $state<string[]>(_sp.tags ?? []);
  let category = $state<string>(_sp.category ?? 'other');
  let scope = $state<string>(_sp.scope ?? '');
  let focal_modifier = $state<string>(_sp.focal_modifier ?? '');
  let mount = $state<string>(_sp.mount ?? '');
  // Structured filter chips — replaces the legacy free-text filters field.
  let filterChips = $state<PhotoFilterChip[]>(_filterChips);
  // Derived legacy string for SetupPicker conflict detection.
  let filtersString = $derived(filterChips.map((f) => f.display_name).join(', '));
  let guiding = $state<string>(_sp.guiding ?? '');
  let photo_setup_id = $state<string | null>(_sp.setup_id ?? null);

  async function onApplySetup(req: { setup_id: string; mode: 'fill_empty' | 'overwrite' }) {
    const r = await fetch(`${API}/api/photos/${data.photo.id}/apply-setup`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify(req)
    });
    if (!r.ok) return;
    const out = await r.json();
    // The backend returns the canonical denormalized columns; sync them
    // back into the form state so the user sees the change immediately.
    scope = out.scope ?? '';
    focal_modifier = out.focal_modifier ?? '';
    camera = out.camera ?? '';
    mount = out.mount ?? '';
    filterChips = (out.filter_items as PhotoFilterChip[] | undefined) ?? filterChips;
    guiding = out.guiding ?? '';
    photo_setup_id = out.setup_id ?? null;
  }

  async function onDetachSetup() {
    const r = await fetch(`${API}/api/photos/${data.photo.id}/detach-setup`, {
      method: 'POST',
      credentials: 'include'
    });
    if (r.ok) {
      photo_setup_id = null;
      // Denormalized columns intentionally NOT cleared per spec.
    }
  }

  let isPublished = $derived(!data.photo.is_draft);
  let isProcessing = $derived(data.photo.status === 'processing');
  let isFailed = $derived(data.photo.status === 'failed');

  // Count fields the upload pipeline recovered from EXIF for the
  // "● N fields recovered" badge in the design handoff.
  let recoveredCount = $derived.by(() => {
    const p = data.photo;
    let n = 0;
    if (p.target) n += 1;
    if (p.taken_at) n += 1;
    if (p.camera) n += 1;
    if (p.lens) n += 1;
    if (p.iso != null) n += 1;
    if (p.exposure_s != null) n += 1;
    if (p.focal_mm != null) n += 1;
    if (p.aperture_f != null) n += 1;
    if (p.ra_deg != null && p.dec_deg != null) n += 1;
    return n;
  });
  let dimensionLabel = $derived(
    data.photo.width && data.photo.height ? `${data.photo.width} × ${data.photo.height}` : null
  );
  let bytesLabel = $derived(
    Number(data.photo.bytes) ? `${(Number(data.photo.bytes) / (1024 * 1024)).toFixed(1)} MB` : null
  );

  $effect(() => {
    if (isProcessing && polling === null) {
      polling = window.setInterval(() => invalidateAll(), 2000);
    }
    if (!isProcessing && polling !== null) {
      clearInterval(polling);
      polling = null;
    }
    return () => {
      if (polling !== null) clearInterval(polling);
    };
  });
</script>

<svelte:head><title>Verify data — Astrophoto</title></svelte:head>
<AppHeader active="Gallery" />

<div class="verify-page">
  <div class="t-eyebrow">{isPublished ? 'EDIT METADATA' : 'NEW FRAME'}</div>
  <h1 class="title">Verify the <em>data</em>.</h1>
  {#if !isPublished}
    <UploadStepper currentStep={2} />
    <p class="lede">
      We've read what your file knew. Correct anything that's wrong — every field is editable, none
      are required.
    </p>
  {/if}

  {#if isFailed}
    <div class="panel-failed">
      <div class="t-eyebrow danger">
        ● UPLOAD FAILED · {data.photo.pipeline_error ?? 'unknown error'}
      </div>
      <div class="actions">
        <form method="POST" action="?/save_draft">
          <Button variant="ghost" type="submit">Discard</Button>
        </form>
        <Button variant="primary" href="/upload">Retry upload</Button>
      </div>
    </div>
  {:else}
    <div class="layout">
      <aside class="preview" aria-label="Your upload">
        <div class="t-label">YOUR UPLOAD</div>
        <div class="preview-frame">
          {#if isProcessing}
            <div class="processing-overlay">
              <div class="processing-eyebrow">● PROCESSING THUMBNAILS</div>
              <div class="processing-bar" aria-hidden="true"><span></span></div>
            </div>
          {/if}
          <Img
            photoId={data.photo.id}
            w={1200}
            alt={data.photo.target ?? data.photo.original_name}
            class="preview-img"
          />
        </div>
        <div class="preview-meta">
          <span class="filename">{data.photo.original_name}</span>
          <span class="dim">
            {#if bytesLabel}{bytesLabel}{/if}
            {#if bytesLabel && dimensionLabel}
              ·
            {/if}
            {#if dimensionLabel}{dimensionLabel}{/if}
          </span>
        </div>
        <table class="exif">
          <tbody>
            <tr><th>Camera</th><td class="mono">{data.photo.camera ?? '—'}</td></tr>
            <tr><th>ISO</th><td class="mono">{data.photo.iso ?? '—'}</td></tr>
            <tr
              ><th>Sub exposure</th><td class="mono">
                {data.photo.exposure_s != null ? `${data.photo.exposure_s} s` : '—'}
              </td></tr
            >
            <tr><th>Gain</th><td class="mono">{data.photo.gain ?? '—'}</td></tr>
            <tr
              ><th>Sensor temp</th><td class="mono">
                {data.photo.sensor_temp_c != null ? `${data.photo.sensor_temp_c} °C` : '—'}
              </td></tr
            >
            <tr><th>Frames captured</th><td class="mono">{data.photo.sessions ?? '—'}</td></tr>
          </tbody>
        </table>
        {#if !isPublished}
          <a class="replace-link" href="/upload" data-sveltekit-reload>← Replace file</a>
        {/if}
      </aside>

      <form
        method="POST"
        action={isPublished ? '?/save_changes_published' : '?/save_continue'}
        class="metadata-form"
      >
        <div class="form-status">
          <span class="t-label">DETECTED FROM YOUR FILE</span>
          <span class="t-meta status-accent">
            ● {recoveredCount === 0
              ? 'No EXIF fields detected'
              : `${recoveredCount} field${recoveredCount === 1 ? '' : 's'} recovered from EXIF`}
          </span>
        </div>
        <fieldset disabled={isProcessing}>
          <!-- Row 1: target + category (full-width each) -->
          <div class="field-full">
            <TargetPicker bind:value={target} />
          </div>

          <div class="field-full">
            <CategorySegmented bind:value={category} />
          </div>

          <!-- Row 2: numeric EXIF fields in 2-col grid -->
          <div class="t-label section-label">ACQUISITION &amp; FRAMING</div>
          <div class="grid">
            <label>
              <span class="t-label">LENS</span>
              <Input name="lens" value={data.photo.lens ?? ''} />
            </label>
            <label>
              <span class="t-label">ISO</span>
              <Input type="number" name="iso" value={data.photo.iso?.toString() ?? ''} />
            </label>
            <label>
              <span class="t-label">EXPOSURE (S)</span>
              <Input
                type="number"
                step="0.01"
                name="exposure_s"
                value={data.photo.exposure_s?.toString() ?? ''}
              />
            </label>
            <label>
              <span class="t-label">FOCAL (MM)</span>
              <Input type="number" name="focal_mm" value={data.photo.focal_mm?.toString() ?? ''} />
            </label>
            <label>
              <span class="t-label">APERTURE (f/)</span>
              <Input
                type="number"
                step="0.1"
                name="aperture_f"
                value={data.photo.aperture_f?.toString() ?? ''}
              />
            </label>
            <label>
              <span class="t-label">SESSIONS</span>
              <Input type="number" name="sessions" value={data.photo.sessions?.toString() ?? ''} />
            </label>
            <label>
              <span class="t-label">GAIN</span>
              <Input type="number" name="gain" value={data.photo.gain?.toString() ?? ''} />
            </label>
            <label>
              <span class="t-label">SENSOR TEMP (°C)</span>
              <Input
                type="number"
                step="0.1"
                name="sensor_temp_c"
                value={data.photo.sensor_temp_c?.toString() ?? ''}
              />
            </label>
            <label>
              <span class="t-label">RA (DEG)</span>
              <Input
                type="number"
                step="0.0001"
                name="ra_deg"
                value={data.photo.ra_deg?.toString() ?? ''}
              />
            </label>
            <label>
              <span class="t-label">DEC (DEG)</span>
              <Input
                type="number"
                step="0.0001"
                name="dec_deg"
                value={data.photo.dec_deg?.toString() ?? ''}
              />
            </label>
          </div>

          <!-- Side-channel plate-solve: upload an XISF master, the server
               forwards it to platesolve.astrophoto.pics and writes the
               WCS result onto the photo row. XISF is NOT stored. -->
          <div class="plate-solve">
            <span class="t-label plate-solve-label">OPTIONAL · PLATE SOLVE</span>
            <p class="plate-solve-body">
              Upload an XISF master to recover RA/Dec, scale, and rotation precisely. Takes ~30 s.
              The XISF is not stored — only the solved coordinates.
            </p>

            {#if psState === 'solving'}
              <p class="t-meta plate-solve-progress">
                ● SOLVING — polling every {PLATESOLVE_POLL_INTERVAL_MS / 1000} s
              </p>
            {:else if psState === 'solved' && psStatus}
              <p class="t-meta plate-solve-solved">● SOLVED · {formatTelemetry(psStatus)}</p>
            {:else if psState === 'failed' && psStatus?.error}
              <p class="t-meta plate-solve-failed">● FAILED · {psStatus.error}</p>
            {/if}

            <div class="plate-solve-controls">
              <label class="plate-solve-file">
                <span class="t-label">XISF FILE</span>
                <input
                  type="file"
                  accept=".xisf,application/x-xisf"
                  disabled={psIsSolving || psSubmitting}
                  onchange={onPickXisf}
                />
              </label>
              <Button
                variant="primary"
                type="button"
                disabled={!psFile || psSubmitting || psIsSolving}
                onclick={onCalibrate}
              >
                {#if psSubmitting}
                  Uploading…
                {:else if psIsSolving}
                  Solving…
                {:else if psState === 'solved'}
                  Re-calibrate
                {:else if psState === 'failed'}
                  Retry
                {:else}
                  Calibrate
                {/if}
              </Button>
            </div>
            {#if psLocalError}
              <p class="t-meta plate-solve-failed">{psLocalError}</p>
            {/if}
          </div>

          <!-- Row 3: equipment pickers in 2-col grid -->
          <div class="t-label section-label">EQUIPMENT</div>
          <div class="setup-row">
            <SetupPicker
              setups={data.setups}
              currentSetupId={photo_setup_id}
              current={{ scope, focal_modifier, camera, mount, filters: filtersString, guiding }}
              onapply={onApplySetup}
              ondetach={onDetachSetup}
            />
          </div>
          <div class="grid equipment-grid">
            <div class="field">
              <EquipmentAutocomplete name="camera" kind="camera" bind:value={camera} />
            </div>
            <div class="field">
              <EquipmentAutocomplete name="scope" kind="telescope" bind:value={scope} />
            </div>
            <div class="field">
              <EquipmentAutocomplete
                name="focal_modifier"
                kind="focal_modifier"
                bind:value={focal_modifier}
              />
            </div>
            <div class="field">
              <EquipmentAutocomplete name="mount" kind="mount" bind:value={mount} />
            </div>
            <div class="field field-filters-full">
              <span class="t-label">FILTERS · STRUCTURED</span>
              <FilterChipInput
                value={filterChips}
                orphans={data.orphans}
                startOpen={!isPublished}
                onChange={(next) => (filterChips = next)}
              />
              <input
                type="hidden"
                name="filter_item_ids"
                value={filterChips.map((f) => f.id).join(',')}
              />
            </div>
            <div class="field">
              <EquipmentAutocomplete name="guiding" kind="guiding" bind:value={guiding} />
            </div>
          </div>

          <!-- Row 4: tags (full width) -->
          <div class="field-full">
            <TagInput bind:value={tags} />
          </div>
        </fieldset>

        {#if isProcessing}
          <p class="t-meta">● PROCESSING THUMBNAILS — polling every 2 s</p>
        {/if}
        {#if form?.error}
          <p class="t-meta form-error">{form.error}</p>
        {/if}

        <div class="actions">
          {#if isPublished}
            <Button variant="ghost" href="/upload/{data.photo.id}/caption">Edit caption →</Button>
            <Button variant="primary" type="submit" disabled={isProcessing}>Save changes</Button>
          {:else}
            <Button variant="ghost" type="submit" formaction="?/save_draft" disabled={isProcessing}
              >Save as draft</Button
            >
            <Button variant="primary" type="submit" disabled={isProcessing}>Continue →</Button>
          {/if}
        </div>
      </form>
    </div>
  {/if}
</div>

<style>
  .verify-page {
    padding: 40px 64px 64px;
    max-width: 1440px;
    margin: 0 auto;
  }
  .title {
    font-family: var(--font-display);
    font-size: 44px;
    margin: 8px 0 12px;
  }
  .title em {
    font-style: italic;
  }
  .lede {
    color: var(--fg-secondary);
    font-size: 13px;
    max-width: 64ch;
    margin: 24px 0 32px;
  }
  .layout {
    display: grid;
    grid-template-columns: 520px 1fr;
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
  .processing-overlay {
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: 12px;
    z-index: 1;
    padding: 8px 12px;
    background: rgba(12, 10, 8, 0.85);
    border: 1px solid var(--border-default);
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-secondary);
  }
  .processing-eyebrow {
    color: var(--accent);
  }
  .processing-bar {
    margin-top: 6px;
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
  .preview-meta {
    display: flex;
    justify-content: space-between;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
    letter-spacing: 0.04em;
  }
  .exif {
    margin-top: 20px;
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }
  .exif th {
    text-align: left;
    font-family: var(--font-mono);
    font-weight: 400;
    color: var(--fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: 11px;
    padding: 6px 16px 6px 0;
    border-bottom: 1px solid var(--border-subtle);
    vertical-align: top;
    white-space: nowrap;
  }
  .exif td {
    padding: 6px 0;
    border-bottom: 1px solid var(--border-subtle);
    color: var(--fg-secondary);
  }
  .exif td.mono {
    font-family: var(--font-mono);
  }
  .exif tr:last-child th,
  .exif tr:last-child td {
    border-bottom: none;
  }
  .filename {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 60%;
  }
  .replace-link {
    align-self: flex-start;
    color: var(--fg-secondary);
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.04em;
    text-decoration: none;
    margin-top: 8px;
    border: 1px solid var(--border-default);
    padding: 6px 12px;
  }
  .replace-link:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
  .form-status {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 16px;
    gap: 16px;
  }
  .status-accent {
    color: var(--accent);
  }
  .plate-solve {
    margin-top: 16px;
    margin-bottom: 24px;
    padding: 16px;
    border: 1px solid var(--border-default);
    background: var(--bg-base, var(--bg-canvas));
  }
  .plate-solve-label {
    color: var(--accent);
    display: block;
    margin-bottom: 8px;
  }
  .plate-solve-body {
    margin: 0;
    font-size: 12px;
    color: var(--fg-secondary);
  }
  .plate-solve-progress {
    margin: 12px 0 0;
    color: var(--accent);
  }
  .plate-solve-solved {
    margin: 12px 0 0;
    color: var(--accent);
  }
  .plate-solve-failed {
    margin: 12px 0 0;
    color: var(--danger);
  }
  .plate-solve-controls {
    display: flex;
    align-items: flex-end;
    gap: 12px;
    margin-top: 12px;
  }
  .plate-solve-file {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
    min-width: 0;
  }
  .plate-solve-file input[type='file'] {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-secondary);
  }
  .field-full {
    margin-bottom: 16px;
  }
  .section-label {
    margin: 24px 0 12px;
    color: var(--fg-muted);
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
  .equipment-grid .field-filters-full {
    grid-column: 1 / -1;
  }
  .setup-row {
    margin-bottom: 1rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border, #ccc);
  }
  .actions {
    display: flex;
    gap: 12px;
    justify-content: flex-end;
    margin-top: 32px;
  }
  .panel-failed {
    padding: 24px;
    border: 1px solid var(--danger);
    margin-top: 32px;
  }
  .danger {
    color: var(--danger);
  }
  .form-error {
    color: var(--danger);
  }
  @media (max-width: 1024px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }
  @media (max-width: 768px) {
    .verify-page {
      padding: 32px 24px;
    }
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
