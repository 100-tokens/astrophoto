<script lang="ts">
  import FieldShell from './FieldShell.svelte';

  // TextField — the standard label+input cell used by every numeric and
  // free-text input in the main column of the verify form. See design
  // handoff §"Field primitive". Notable details:
  //   - `mono` toggles between Inter and JetBrains Mono (default mono on)
  //   - `suffix` (e.g. "mm", "s", "°C") renders absolutely inside the input
  //     and reserves right-side padding so the value doesn't overlap.
  //   - `numeric` switches to inputmode=decimal (NOT type=number — that
  //     drops the browser spinner that throws off both touch keyboards
  //     and visual alignment).
  //   - `aria-invalid` is intentionally NOT set while typing. Validation is
  //     a blur-only concern in this form per the brief.
  interface Props {
    name?: string;
    label?: string | null;
    value?: string;
    placeholder?: string;
    mono?: boolean;
    detected?: boolean;
    /** Explicit provenance; takes precedence over `detected` in FieldShell.
        'setup' → ● FROM SETUP, 'exif' → ● FROM EXIF, null → no chip. */
    source?: 'exif' | 'setup' | null;
    hint?: string | null;
    suffix?: string | null;
    full?: boolean;
    span?: number;
    numeric?: boolean;
    step?: string;
    disabled?: boolean;
    onInput?: (value: string) => void;
  }

  let {
    name,
    label,
    value = $bindable(''),
    placeholder,
    mono = true,
    detected = false,
    source = null,
    hint = null,
    suffix = null,
    full = false,
    span = 1,
    numeric = false,
    step,
    disabled = false,
    onInput
  }: Props = $props();

  function onInputEvent(e: Event) {
    const t = e.target as HTMLInputElement;
    value = t.value;
    onInput?.(t.value);
  }
</script>

<FieldShell {label} {hint} {detected} {source} {full} {span}>
  <div class="tf-wrap">
    <input
      class={'input ' + (mono ? 'input-mono' : '')}
      class:has-suffix={!!suffix}
      type="text"
      inputmode={numeric ? 'decimal' : undefined}
      {name}
      {value}
      {placeholder}
      {step}
      {disabled}
      oninput={onInputEvent}
      autocomplete="off"
      spellcheck={mono ? false : undefined}
    />
    {#if suffix}
      <span class="tf-suffix" aria-hidden="true">{suffix}</span>
    {/if}
  </div>
</FieldShell>

<style>
  .tf-wrap {
    position: relative;
  }
  .tf-wrap .input.has-suffix {
    padding-right: 48px;
  }
  .tf-suffix {
    position: absolute;
    right: 12px;
    top: 0;
    bottom: 0;
    display: inline-flex;
    align-items: center;
    color: var(--fg-faint);
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    pointer-events: none;
  }
</style>
