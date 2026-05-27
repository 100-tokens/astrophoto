<script lang="ts">
  import TextField from './TextField.svelte';
  import FieldShell from './FieldShell.svelte';
  import { parseRaToDeg, parseDecToDeg, formatRaHms, formatDecDms } from '$lib/utils/coords';

  // AcquisitionGrid — 10 numeric/freetext fields in a 4-col grid. RA/DEC
  // span two cells each. Every field is optional; the backend stores nulls
  // when blank. The new TextField primitive uses inputmode=decimal (NOT
  // type=number) so the browser spinner is gone and the aria-invalid-on-
  // typing bug from the previous implementation is structurally avoided.
  // Because the inputs are type=text, the `step` HTML attribute would be
  // a no-op — we drop it and rely on the server's Number() coercion.
  //
  // RA / DEC are a special case. The bound `ra_deg`/`dec_deg` values are
  // decimal-degree strings (server contract, see +page.server.ts::collectPatch
  // → numOrNull). The visible <input>, however, accepts and displays
  // sexagesimal (HMS for RA, DMS for DEC) as well as bare decimals. To
  // keep the server contract intact, the visible field is name-less and a
  // hidden <input name="ra_deg|dec_deg"> carries the decimal canonical
  // value for FormData. On blur, the typed string is parsed and the
  // canonical decimal-degree value is bubbled back via the bound prop;
  // the display is then re-formatted from the canonical.

  interface Props {
    lens?: string;
    iso?: string;
    exposure_s?: string;
    focal_mm?: string;
    aperture_f?: string;
    gain?: string;
    sensor_temp_c?: string;
    sessions?: string;
    ra_deg?: string;
    dec_deg?: string;
    /** Set of field keys whose initial value came from EXIF / plate-solve. */
    fromExif?: Set<string>;
    /** Field keys whose value came from the applied setup (FRAMING:
        focal_mm / aperture_f derived from the optical train). */
    fromSetup?: Set<string>;
    disabled?: boolean;
  }

  let {
    lens = $bindable(''),
    iso = $bindable(''),
    exposure_s = $bindable(''),
    focal_mm = $bindable(''),
    aperture_f = $bindable(''),
    gain = $bindable(''),
    sensor_temp_c = $bindable(''),
    sessions = $bindable(''),
    ra_deg = $bindable(''),
    dec_deg = $bindable(''),
    fromExif = new Set<string>(),
    fromSetup = new Set<string>(),
    disabled = false
  }: Props = $props();

  function has(k: string) {
    return fromExif.has(k);
  }
  // FRAMING fields (focal_mm/aperture_f) can be setup-derived; the rest are
  // EXIF/solve-only. setup wins over exif for the chip.
  function sourceFor(k: string): 'exif' | 'setup' | null {
    if (fromSetup.has(k)) return 'setup';
    if (fromExif.has(k)) return 'exif';
    return null;
  }

  // --- RA / DEC display layer -------------------------------------------
  //
  // `*Display` mirrors what the user sees in the input. While focused, it
  // tracks raw keystrokes. While unfocused, it's the canonical sexagesimal
  // formatting of the bound decimal — so a parent updating ra_deg (e.g.
  // after plate-solve completes) flows through to the visible input.

  function fmtRa(decimalStr: string): string {
    if (decimalStr === '') return '';
    const n = Number(decimalStr);
    return Number.isFinite(n) ? formatRaHms(n) : decimalStr;
  }
  function fmtDec(decimalStr: string): string {
    if (decimalStr === '') return '';
    const n = Number(decimalStr);
    return Number.isFinite(n) ? formatDecDms(n) : decimalStr;
  }

  let raFocused = $state(false);
  let decFocused = $state(false);
  let raTyping = $state<string | null>(null);
  let decTyping = $state<string | null>(null);
  let raBlurred = $state(false);
  let decBlurred = $state(false);
  let raInvalid = $state(false);
  let decInvalid = $state(false);

  let raDisplay = $derived(raFocused && raTyping !== null ? raTyping : fmtRa(ra_deg));
  let decDisplay = $derived(decFocused && decTyping !== null ? decTyping : fmtDec(dec_deg));

  function onRaInput(e: Event) {
    raTyping = (e.target as HTMLInputElement).value;
    // Don't compute aria-invalid while typing per spec — only on blur.
  }
  function onRaFocus() {
    raFocused = true;
    raTyping = fmtRa(ra_deg);
  }
  function onRaBlur() {
    raBlurred = true;
    const raw = raTyping ?? '';
    raFocused = false;
    raTyping = null;
    if (raw.trim() === '') {
      ra_deg = '';
      raInvalid = false;
      return;
    }
    const parsed = parseRaToDeg(raw);
    if (parsed === null) {
      // Keep the user's text on screen so they can fix it. Mark invalid.
      raTyping = raw;
      raFocused = true;
      raInvalid = true;
      return;
    }
    raInvalid = false;
    ra_deg = String(parsed);
  }

  function onDecInput(e: Event) {
    decTyping = (e.target as HTMLInputElement).value;
  }
  function onDecFocus() {
    decFocused = true;
    decTyping = fmtDec(dec_deg);
  }
  function onDecBlur() {
    decBlurred = true;
    const raw = decTyping ?? '';
    decFocused = false;
    decTyping = null;
    if (raw.trim() === '') {
      dec_deg = '';
      decInvalid = false;
      return;
    }
    const parsed = parseDecToDeg(raw);
    if (parsed === null) {
      decTyping = raw;
      decFocused = true;
      decInvalid = true;
      return;
    }
    decInvalid = false;
    dec_deg = String(parsed);
  }
</script>

<div class="acq-head">
  <div class="t-label">ACQUISITION &amp; FRAMING</div>
  <span class="t-meta acq-head-meta">10 fields · numeric</span>
</div>
<div class="acq-grid">
  <TextField name="lens" label="LENS" bind:value={lens} detected={has('lens')} {disabled} />
  <TextField
    name="focal_mm"
    label="FOCAL"
    bind:value={focal_mm}
    suffix="mm"
    numeric
    source={sourceFor('focal_mm')}
    {disabled}
  />
  <TextField
    name="aperture_f"
    label="APERTURE"
    bind:value={aperture_f}
    numeric
    source={sourceFor('aperture_f')}
    {disabled}
  />
  <TextField
    name="iso"
    label="ISO"
    bind:value={iso}
    numeric
    detected={has('iso')}
    placeholder="auto / N/A"
    {disabled}
  />
  <TextField
    name="exposure_s"
    label="EXPOSURE"
    bind:value={exposure_s}
    suffix="s"
    numeric
    detected={has('exposure_s')}
    {disabled}
  />
  <TextField name="gain" label="GAIN" bind:value={gain} numeric detected={has('gain')} {disabled} />
  <TextField
    name="sensor_temp_c"
    label="SENSOR TEMP"
    bind:value={sensor_temp_c}
    suffix="°C"
    numeric
    detected={has('sensor_temp_c')}
    {disabled}
  />
  <TextField
    name="sessions"
    label="SESSIONS"
    bind:value={sessions}
    numeric
    placeholder="nights"
    detected={has('sessions')}
    {disabled}
  />

  <!-- RA: HMS-aware display with a hidden decimal-degree carrier. -->
  <FieldShell
    label="RA"
    hint="J2000 · HMS or decimal degrees · auto from solve"
    detected={has('ra_deg')}
    span={2}
  >
    <input
      class="input input-mono"
      type="text"
      inputmode="decimal"
      autocomplete="off"
      spellcheck={false}
      placeholder="20h 59m 17.2s"
      value={raDisplay}
      aria-invalid={raBlurred && raInvalid ? 'true' : undefined}
      {disabled}
      oninput={onRaInput}
      onfocus={onRaFocus}
      onblur={onRaBlur}
    />
    <input type="hidden" name="ra_deg" value={ra_deg} />
  </FieldShell>

  <!-- DEC: DMS-aware display with a hidden decimal-degree carrier. -->
  <FieldShell
    label="DEC"
    hint="J2000 · DMS or decimal degrees · auto from solve"
    detected={has('dec_deg')}
    span={2}
  >
    <input
      class="input input-mono"
      type="text"
      inputmode="decimal"
      autocomplete="off"
      spellcheck={false}
      placeholder="+44° 31′ 44.0″"
      value={decDisplay}
      aria-invalid={decBlurred && decInvalid ? 'true' : undefined}
      {disabled}
      oninput={onDecInput}
      onfocus={onDecFocus}
      onblur={onDecBlur}
    />
    <input type="hidden" name="dec_deg" value={dec_deg} />
  </FieldShell>
</div>

<style>
  .acq-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 16px;
  }
  .acq-head-meta {
    color: var(--fg-faint);
  }
  .acq-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 24px;
  }
  @media (max-width: 1024px) {
    .acq-grid {
      grid-template-columns: repeat(2, 1fr);
    }
  }
  @media (max-width: 380px) {
    .acq-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
