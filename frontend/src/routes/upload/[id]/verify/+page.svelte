<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import Button from '$lib/components/Button.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import AcquisitionGrid from '$lib/components/verify-form/AcquisitionGrid.svelte';
  import FilterIntegration from '$lib/components/verify-form/FilterIntegration.svelte';
  import CategoryRadio from '$lib/components/verify-form/CategoryRadio.svelte';
  import EquipmentSection from '$lib/components/verify-form/EquipmentSection.svelte';
  import FooterActions from '$lib/components/verify-form/FooterActions.svelte';
  import PlateSolveBlock from '$lib/components/verify-form/PlateSolveBlock.svelte';
  import TagChipInput from '$lib/components/verify-form/TagChipInput.svelte';
  import TargetField from '$lib/components/verify-form/TargetField.svelte';
  import VerifyAside from '$lib/components/verify-form/VerifyAside.svelte';
  import VerifyHero from '$lib/components/verify-form/VerifyHero.svelte';
  import VerifyStepper from '$lib/components/verify-form/VerifyStepper.svelte';
  import { computeProvenance } from '$lib/utils/provenance';
  import type { FilterIntegration as FilterIntegrationT } from '$lib/api/FilterIntegration';
  import type { PhotoFilterChip } from '$lib/api/PhotoFilterChip';
  import type { PlatesolveStatus } from '$lib/api/PlatesolveStatus';
  import type { SetupSummary } from '$lib/api/SetupSummary';
  import type { XisfDisplayMeta } from '$lib/api/XisfDisplayMeta';
  import type { PageProps } from './$types';

  const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '';
  // Mirrors backend/src/photos/platesolve.rs::MAX_XISF_BYTES.
  const PLATESOLVE_MAX_BYTES = 128 * 1024 * 1024;
  const PLATESOLVE_POLL_INTERVAL_MS = 2000;

  let { data, form }: PageProps = $props();
  // Silence unused-import lint: SetupSummary referenced via prop type only.
  type _SetupSummary = SetupSummary;
  let processingPoll = $state<number | null>(null);

  // --------------------------------------------------------------------
  // Plate-solve state machine (preserved verbatim from prior implementation)
  // --------------------------------------------------------------------
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

  // XISF processing-history sidebar — lazy-fetched once when the solve
  // completes. xisfMetaLoaded dedupes concurrent calls.
  let xisfMeta = $state<XisfDisplayMeta | null>(null);
  let xisfMetaLoaded = $state(false);

  async function fetchXisfMeta() {
    if (xisfMetaLoaded) return;
    xisfMetaLoaded = true;
    try {
      const r = await fetch(`${API}/api/photos/${data.photo.id}/xisf-meta`, {
        credentials: 'include'
      });
      if (!r.ok) {
        xisfMetaLoaded = false;
        return;
      }
      xisfMeta = (await r.json()) as XisfDisplayMeta;
    } catch {
      xisfMetaLoaded = false;
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
      body.append('file', psFile);
      const r = await fetch(`${API}/api/photos/${data.photo.id}/platesolve`, {
        method: 'POST',
        credentials: 'include',
        body
      });
      if (r.status === 202 || r.status === 409) {
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

  $effect(() => {
    if (psIsSolving) startPolling();
    else stopPolling();
    return () => stopPolling();
  });

  $effect(() => {
    if (psState === 'solved') void fetchXisfMeta();
  });

  // --------------------------------------------------------------------
  // Form state — every editable field, seeded ONCE from the server load.
  // The cast and helper-function pattern dodges the "$state initialized
  // from prop" lint while keeping the seeding intentional and readable.
  // --------------------------------------------------------------------
  type ShowcasePhoto = typeof data.photo & {
    setup_id?: string | null;
    category?: string | null;
    scope?: string | null;
    focal_modifier?: string | null;
    mount?: string | null;
    filters?: string | null;
    guiding?: string | null;
  };
  function initialPhoto() {
    return data.photo as ShowcasePhoto;
  }
  function initialFilterChips() {
    return (data.photo.filter_items ?? []) as PhotoFilterChip[];
  }
  const _sp = initialPhoto();
  const _fc = initialFilterChips();

  let target = $state<string>(_sp.target ?? '');
  // Prose description shown on the published photo. The dedicated /caption
  // step was removed (commit 56acf4e); verify is now the single publish step,
  // so the caption lives here. Seeded via `_sp` like every other field, which
  // dodges the "$state initialized from prop" lint (see the note above).
  let caption = $state<string>(_sp.caption ?? '');
  // Default 'other' (matches CategorySegmented's prior behavior). The visible
  // segmented row highlights nothing in this state — the user must click DSO
  // / Planetary / etc. to commit, or the discrete "Other" link below.
  let category = $state<string>(_sp.category ?? 'other');
  // Text-encoded numerics — TextField uses inputmode=decimal, the server
  // coerces back to number via Number(). Strings here let blank ↔ null
  // round-trip without special-casing 0.
  let lens = $state<string>(_sp.lens ?? '');
  let iso = $state<string>(_sp.iso != null ? String(_sp.iso) : '');
  let exposure_s = $state<string>(_sp.exposure_s != null ? String(_sp.exposure_s) : '');
  let focal_mm = $state<string>(_sp.focal_mm != null ? String(_sp.focal_mm) : '');
  let aperture_f = $state<string>(_sp.aperture_f != null ? String(_sp.aperture_f) : '');
  let gain = $state<string>(_sp.gain != null ? String(_sp.gain) : '');
  let sensor_temp_c = $state<string>(_sp.sensor_temp_c != null ? String(_sp.sensor_temp_c) : '');
  let sessions = $state<string>(_sp.sessions != null ? String(_sp.sessions) : '');
  let ra_deg = $state<string>(_sp.ra_deg != null ? String(_sp.ra_deg) : '');
  let dec_deg = $state<string>(_sp.dec_deg != null ? String(_sp.dec_deg) : '');
  let camera = $state<string>(_sp.camera ?? '');
  let scope = $state<string>(_sp.scope ?? '');
  let focal_modifier = $state<string>(_sp.focal_modifier ?? '');
  let mount = $state<string>(_sp.mount ?? '');
  let guiding = $state<string>(_sp.guiding ?? '');
  let filterChips = $state<PhotoFilterChip[]>(_fc);
  let tags = $state<string[]>(_sp.tags ?? []);
  let filterIntegrations = $state<FilterIntegrationT[]>(_sp.filter_integrations ?? []);
  let photo_setup_id = $state<string | null>(_sp.setup_id ?? null);

  // Plate-solve-OWNED fields: ra_deg, dec_deg, focal_mm, aperture_f. The
  // solver measures these authoritatively (save_result writes them), so a
  // *new* solve must replace whatever the form currently holds — including a
  // theoretical focal_mm an applied setup wrote earlier (e.g. 2032 from an
  // EdgeHD 8 setup, which the solve corrects to the measured 1466.3).
  //
  // Why this matters: the autosave PUT sends every field, and the backend
  // metadata handler writes any key present in the body (the CASE boolean is
  // `is_some()`, i.e. key-present, not value-non-null — see
  // backend/src/photos/metadata.rs). So if local form state still held the
  // stale setup focal_mm after a solve, the next autosave would clobber the
  // freshly-measured value straight back to the theoretical one. We keep
  // local state in lock-step with the solve instead of guarding the backend
  // (a backend guard would break legitimate manual nulling — see the
  // verify_metadata contract tests).
  //
  // Keyed on platesolveStatus.solvedAt: it changes only when a genuinely new
  // solve lands (invalidateAll re-runs the load), so a manual edit between
  // solves survives — re-solving is the one action that overrides it.
  function initialSolvedAt(): string | null {
    return data.platesolveStatus?.solvedAt ?? null;
  }
  let lastSyncedSolveAt = $state<string | null>(initialSolvedAt());
  $effect(() => {
    const solvedAt = data.platesolveStatus?.solvedAt ?? null;
    if (!solvedAt || solvedAt === lastSyncedSolveAt) return;
    // A new measurement is authoritative — adopt every solve-owned field.
    if (data.photo.ra_deg != null) ra_deg = String(data.photo.ra_deg);
    if (data.photo.dec_deg != null) dec_deg = String(data.photo.dec_deg);
    if (data.photo.focal_mm != null) focal_mm = String(data.photo.focal_mm);
    if (data.photo.aperture_f != null) aperture_f = String(data.photo.aperture_f);
    lastSyncedSolveAt = solvedAt;
  });

  let filtersString = $derived(filterChips.map((f) => f.display_name).join(', '));

  // --------------------------------------------------------------------
  // Setup apply / detach — preserved from prior implementation.
  // --------------------------------------------------------------------
  async function onApplySetup(req: { setup_id: string; mode: 'fill_empty' | 'overwrite' }) {
    const r = await fetch(`${API}/api/photos/${data.photo.id}/apply-setup`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify(req)
    });
    if (!r.ok) return;
    const out = await r.json();
    scope = out.scope ?? '';
    focal_modifier = out.focal_modifier ?? '';
    camera = out.camera ?? '';
    mount = out.mount ?? '';
    filterChips = (out.filter_items as PhotoFilterChip[] | undefined) ?? filterChips;
    guiding = out.guiding ?? '';
    // FRAMING derived from the optical train (telescope focal × modifier
    // factor, focal ÷ aperture). Reflect the backend's just-written values
    // so FOCAL/APERTURE populate without a reload.
    if (out.focal_mm != null) focal_mm = String(out.focal_mm);
    if (out.aperture_f != null) aperture_f = String(out.aperture_f);
    photo_setup_id = out.setup_id ?? null;
  }
  async function onDetachSetup() {
    const r = await fetch(`${API}/api/photos/${data.photo.id}/detach-setup`, {
      method: 'POST',
      credentials: 'include'
    });
    if (r.ok) {
      photo_setup_id = null;
    }
  }

  // --------------------------------------------------------------------
  // Derived photo state.
  // --------------------------------------------------------------------
  let isPublished = $derived(!data.photo.is_draft);
  let isProcessing = $derived(
    data.photo.status === 'processing' || data.photo.status === 'awaiting-calibration'
  );
  let isAwaitingCalibration = $derived(data.photo.status === 'awaiting-calibration');
  let isFailed = $derived(data.photo.status === 'failed');

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

  // Per-field provenance for the ● FROM EXIF / FROM SETUP chips. An
  // equipment value matching the applied setup is FROM SETUP; mount /
  // focal_modifier / guiding are never FROM EXIF (not in a file header);
  // acquisition scalars are EXIF/solve-sourced when present. See
  // computeProvenance for the full rules.
  // A successful plate-solve makes focal/aperture/RA/Dec measured ground
  // truth → FROM SOLVE (spec B). The load already fetched the solve status.
  let solved = $derived(data.platesolveStatus?.state === 'solved');
  let provenance = $derived(
    computeProvenance(data.photo as ShowcasePhoto, data.setupValues, { solved })
  );
  let fromExif = $derived(provenance.fromExif);
  let fromSetup = $derived(provenance.fromSetup);
  let fromSolve = $derived(provenance.fromSolve);

  let appliedSpec = $derived.by(() => {
    if (!photo_setup_id) return null;
    const summary = [camera, scope, ...filterChips.map((f) => f.display_name)]
      .filter(Boolean)
      .slice(0, 3)
      .join(' · ')
      .toUpperCase();
    const found = (data.setups as SetupSummary[]).find((s) => s.id === photo_setup_id);
    return {
      name: found?.name ?? 'Saved setup',
      summary,
      setupIdShort: `${photo_setup_id.slice(0, 4)}…${photo_setup_id.slice(-2)}`
    };
  });

  // --------------------------------------------------------------------
  // Background processing poll (thumbnails or auto-platesolve).
  // --------------------------------------------------------------------
  $effect(() => {
    if (isProcessing && processingPoll === null) {
      processingPoll = window.setInterval(() => invalidateAll(), 2000);
    }
    if (!isProcessing && processingPoll !== null) {
      clearInterval(processingPoll);
      processingPoll = null;
    }
    return () => {
      if (processingPoll !== null) clearInterval(processingPoll);
    };
  });

  // --------------------------------------------------------------------
  // Autosave — debounced PUT through the same-origin /api proxy. The
  // form-action save path ("Continue →", "Save as draft", "Save changes")
  // remains the canonical commit on navigation; this autosave is purely
  // for resilience between commits.
  //
  // Skipped when the photo is already published (the published-edit path
  // is gated behind the explicit "Save changes" action — silent writes
  // would surprise the user). The first effect run is the seed pass, not
  // a real edit, so a `firstSyncDone` flag suppresses it.
  // --------------------------------------------------------------------
  const AUTOSAVE_DEBOUNCE_MS = 1500;
  type SaveState = 'idle' | 'saving' | 'saved' | 'error';
  let saveState = $state<SaveState>('saved');
  let lastSavedAt = $state<number>(Date.now());
  let now = $state(Date.now());
  let firstSyncDone = $state(false);
  let autosaveAbort: AbortController | null = null;
  $effect(() => {
    const h = window.setInterval(() => {
      now = Date.now();
    }, 1000);
    return () => clearInterval(h);
  });

  function buildAutosavePatch() {
    const numOrNull = (s: string): number | null => {
      if (s.trim() === '') return null;
      const n = Number(s);
      return Number.isFinite(n) ? n : null;
    };
    const strOrNull = (s: string): string | null => {
      const t = s.trim();
      return t === '' ? null : t;
    };
    return {
      target: strOrNull(target),
      caption: strOrNull(caption),
      category: strOrNull(category),
      lens: strOrNull(lens),
      iso: numOrNull(iso),
      exposure_s: numOrNull(exposure_s),
      focal_mm: numOrNull(focal_mm),
      aperture_f: numOrNull(aperture_f),
      gain: numOrNull(gain),
      sensor_temp_c: numOrNull(sensor_temp_c),
      sessions: numOrNull(sessions),
      ra_deg: numOrNull(ra_deg),
      dec_deg: numOrNull(dec_deg),
      camera: strOrNull(camera),
      scope: strOrNull(scope),
      focal_modifier: strOrNull(focal_modifier),
      mount: strOrNull(mount),
      filters: filtersString === '' ? null : filtersString,
      guiding: strOrNull(guiding),
      tags,
      filter_item_ids: filterChips.map((f) => f.id),
      filter_integrations: filterIntegrations,
      last_step: 'verify' as const
    };
  }

  $effect(() => {
    // Reference every editable field so the effect re-runs on any edit.
    void target;
    void caption;
    void category;
    void lens;
    void iso;
    void exposure_s;
    void focal_mm;
    void aperture_f;
    void gain;
    void sensor_temp_c;
    void sessions;
    void ra_deg;
    void dec_deg;
    void camera;
    void scope;
    void focal_modifier;
    void mount;
    void guiding;
    void filterChips;
    void tags;
    void filterIntegrations;
    if (!firstSyncDone) {
      firstSyncDone = true;
      return;
    }
    // Don't autosave a published photo — that path requires explicit
    // confirmation via the "Save changes" form action.
    if (isPublished) return;

    const patch = buildAutosavePatch();
    const handle = window.setTimeout(async () => {
      // Cancel any in-flight save so a stale 200 can't overwrite a
      // newer edit's "saving" state.
      if (autosaveAbort) autosaveAbort.abort();
      const ctrl = new AbortController();
      autosaveAbort = ctrl;
      saveState = 'saving';
      try {
        const r = await fetch(`/api/photos/${data.photo.id}`, {
          method: 'PUT',
          credentials: 'include',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(patch),
          signal: ctrl.signal
        });
        if (ctrl.signal.aborted) return;
        if (r.ok) {
          saveState = 'saved';
          lastSavedAt = Date.now();
        } else {
          saveState = 'error';
        }
      } catch (e) {
        if ((e as { name?: string }).name === 'AbortError') return;
        saveState = 'error';
      } finally {
        if (autosaveAbort === ctrl) autosaveAbort = null;
      }
    }, AUTOSAVE_DEBOUNCE_MS);
    return () => clearTimeout(handle);
  });
  let secondsSinceSaved = $derived((now - lastSavedAt) / 1000);
</script>

<svelte:head><title>Verify data — Astrophoto</title></svelte:head>
<AppHeader active="Gallery" />

<main>
  <div class="verify-page">
    {#if isFailed}
      <section class="hero-band">
        <VerifyHero
          eyebrow="UPLOAD FAILED"
          title="Something went wrong."
          intro="The pipeline couldn't finish processing this frame. Discard the draft to start over, or retry the upload from step one."
        />
      </section>
      <section class="failed-band">
        <div class="panel-failed">
          <div class="t-eyebrow danger">
            ● UPLOAD FAILED · {data.photo.pipeline_error ?? 'unknown error'}
          </div>
          <div class="failed-actions">
            <form method="POST" action="?/save_draft">
              <Button variant="ghost" type="submit">Discard</Button>
            </form>
            <Button variant="primary" href="/upload">Retry upload</Button>
          </div>
        </div>
      </section>
    {:else}
      <section class="hero-band">
        <VerifyHero
          eyebrow={isPublished ? 'EDIT METADATA' : 'NEW FRAME'}
          title="Verify the data."
          intro="Your camera and the plate-solver already wrote down most of this. Glance through, correct anything off, fill what's still empty — none of it is required."
        />
        {#if !isPublished}
          <div class="hero-stepper">
            <VerifyStepper currentStep={2} variant="three" />
          </div>
        {/if}
      </section>

      <section class="body-band">
        <VerifyAside
          photo={data.photo}
          {xisfMeta}
          {isProcessing}
          {isAwaitingCalibration}
          {isPublished}
        />

        <form
          method="POST"
          action={isPublished ? '?/save_changes_published' : '?/publish'}
          class="metadata-form"
        >
          <div class="status-pill-row" aria-live="polite">
            <span class="chip chip-accent status-pill">
              <span class="status-dot" aria-hidden="true"></span>
              {recoveredCount === 0
                ? '0 fields recovered — fill anything you remember'
                : `${recoveredCount} field${recoveredCount === 1 ? '' : 's'} recovered`}
            </span>
            <span class="t-meta status-meta">last edited just now</span>
          </div>

          <fieldset class="form-fieldset" disabled={isProcessing}>
            <div class="block">
              <TargetField bind:value={target} />
            </div>

            <div class="block">
              <CategoryRadio bind:value={category} />
            </div>

            <div class="block">
              <div class="t-label">CAPTION</div>
              <Textarea
                name="caption"
                rows={4}
                bind:value={caption}
                placeholder="Describe the conditions, processing, equipment used…"
              />
            </div>

            <div class="block">
              <AcquisitionGrid
                bind:lens
                bind:iso
                bind:exposure_s
                bind:focal_mm
                bind:aperture_f
                bind:gain
                bind:sensor_temp_c
                bind:sessions
                bind:ra_deg
                bind:dec_deg
                {fromExif}
                {fromSetup}
                {fromSolve}
                perFilter={filterIntegrations.length > 0}
              />
              <FilterIntegration
                value={filterIntegrations}
                catalogFilters={filterChips}
                onChange={(next) => (filterIntegrations = next)}
              />
              <input
                type="hidden"
                name="filter_integrations"
                value={JSON.stringify(filterIntegrations)}
              />
            </div>

            <div class="block">
              <PlateSolveBlock
                status={psStatus}
                file={psFile}
                submitting={psSubmitting}
                localError={psLocalError}
                maxBytes={PLATESOLVE_MAX_BYTES}
                onPick={onPickXisf}
                {onCalibrate}
              />
            </div>

            <div class="block">
              <EquipmentSection
                setups={data.setups}
                currentSetupId={photo_setup_id}
                {appliedSpec}
                bind:camera
                bind:scope
                bind:focal_modifier
                bind:mount
                bind:guiding
                {filtersString}
                {filterChips}
                orphans={data.orphans}
                startFilterOpen={false}
                {fromExif}
                {fromSetup}
                onApply={onApplySetup}
                onDetach={onDetachSetup}
                onChipsChange={(next) => (filterChips = next)}
              />
            </div>

            <div class="block">
              <TagChipInput bind:value={tags} />
            </div>

            <!-- Legacy comma-string filters cache: the server still accepts
               this for back-compat. The structured `filter_item_ids`
               (emitted inside EquipmentSection) takes precedence on the
               server when both are present. -->
            <input type="hidden" name="filters" value={filtersString} />
          </fieldset>

          {#if isProcessing}
            <p class="t-meta processing-meta">
              ● {isAwaitingCalibration ? 'PLATE-SOLVING XISF' : 'PROCESSING THUMBNAILS'} — polling every
              2 s
            </p>
          {/if}
          {#if form?.error}
            <p class="t-meta form-error">{form.error}</p>
          {/if}

          <FooterActions {saveState} {secondsSinceSaved}>
            {#snippet actions()}
              {#if isPublished}
                <Button variant="primary" type="submit" size="lg" disabled={isProcessing}>
                  Save changes
                </Button>
              {:else}
                <Button
                  variant="ghost"
                  size="lg"
                  type="submit"
                  formaction="?/save_draft"
                  disabled={isProcessing}
                >
                  Save as draft
                </Button>
                <Button variant="primary" size="lg" type="submit" disabled={isProcessing}>
                  Publish
                </Button>
              {/if}
            {/snippet}
          </FooterActions>
        </form>
      </section>
    {/if}
  </div>
</main>

<style>
  .verify-page {
    max-width: 1440px;
    margin: 0 auto;
  }
  .hero-band {
    padding: 56px 64px 32px;
  }
  .hero-stepper {
    margin-top: 40px;
  }
  .body-band {
    padding: 24px 64px 80px;
    display: grid;
    grid-template-columns: 440px 1fr;
    gap: 72px;
    align-items: start;
  }

  .status-pill-row {
    margin-bottom: 28px;
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
  }
  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    font-size: 11px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }
  .status-dot {
    width: 6px;
    height: 6px;
    background: var(--accent);
    border-radius: 50%;
    flex: 0 0 6px;
  }
  .status-meta {
    color: var(--fg-faint);
  }

  .form-fieldset {
    border: 0;
    padding: 0;
    margin: 0;
  }
  .form-fieldset[disabled] {
    opacity: 0.6;
  }
  .block {
    margin-bottom: 40px;
  }
  .block:last-of-type {
    margin-bottom: 28px;
  }

  .processing-meta {
    color: var(--accent);
    margin-bottom: 16px;
  }
  .form-error {
    color: var(--danger);
    margin-bottom: 16px;
  }

  .failed-band {
    padding: 0 64px 80px;
  }
  .panel-failed {
    padding: 24px;
    border: 1px solid var(--border-danger);
    border-radius: var(--r-sm);
    background: var(--bg-danger-tint);
  }
  .failed-actions {
    display: flex;
    gap: 12px;
    justify-content: flex-end;
    margin-top: 24px;
  }
  .danger {
    color: var(--danger);
  }

  @media (max-width: 1023px) {
    .body-band {
      grid-template-columns: 1fr;
      gap: 48px;
    }
  }
  @media (max-width: 768px) {
    .hero-band {
      padding: 40px 24px 24px;
    }
    .body-band {
      padding: 16px 24px 56px;
      gap: 32px;
    }
    .failed-band {
      padding: 0 24px 56px;
    }
  }
</style>
