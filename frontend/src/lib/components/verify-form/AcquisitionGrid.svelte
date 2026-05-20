<script lang="ts">
  import TextField from './TextField.svelte';

  // AcquisitionGrid — 10 numeric/freetext fields in a 4-col grid. RA/DEC
  // span two cells each. Every field is optional; the backend stores nulls
  // when blank. The new TextField primitive uses inputmode=decimal (NOT
  // type=number) so the browser spinner is gone and the aria-invalid-on-
  // typing bug from the previous implementation is structurally avoided.
  // Because the inputs are type=text, the `step` HTML attribute would be
  // a no-op — we drop it and rely on the server's Number() coercion.

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
    disabled = false
  }: Props = $props();

  function has(k: string) {
    return fromExif.has(k);
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
    detected={has('focal_mm')}
    {disabled}
  />
  <TextField
    name="aperture_f"
    label="APERTURE"
    bind:value={aperture_f}
    numeric
    detected={has('aperture_f')}
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
  <TextField
    name="ra_deg"
    label="RA"
    bind:value={ra_deg}
    numeric
    span={2}
    detected={has('ra_deg')}
    hint="J2000 · decimal degrees · auto from solve"
    {disabled}
  />
  <TextField
    name="dec_deg"
    label="DEC"
    bind:value={dec_deg}
    numeric
    span={2}
    detected={has('dec_deg')}
    hint="J2000 · decimal degrees · auto from solve"
    {disabled}
  />
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
