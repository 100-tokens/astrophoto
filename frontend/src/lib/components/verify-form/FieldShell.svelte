<script lang="ts">
  import type { Snippet } from 'svelte';

  // FieldShell — the label+optional-from-exif tag+hint container around every
  // form input on the verify page. Matches design handoff §FieldShell:
  //   - label on top in t-label
  //   - optional "● FROM EXIF" mono accent tag right-aligned with the label
  //   - 8px gap to the input
  //   - 6px gap then a t-meta hint underneath
  //
  // `full` makes the field span the full grid; `span` lets a field straddle
  // multiple cells without going full-width (used for RA/DEC at span=2).
  interface Props {
    label?: string | null | undefined;
    hint?: string | null | undefined;
    detected?: boolean;
    full?: boolean;
    span?: number;
    children: Snippet;
    /** Optional extra slot for a right-side label adornment (overrides "FROM EXIF"). */
    rightAdornment?: Snippet;
  }

  let {
    label,
    hint,
    detected = false,
    full = false,
    span = 1,
    children,
    rightAdornment
  }: Props = $props();

  let gridColumn = $derived(full ? '1 / -1' : span > 1 ? `span ${span}` : undefined);
</script>

<label class="field-shell" style:grid-column={gridColumn}>
  {#if label || rightAdornment || detected}
    <div class="field-shell__head">
      {#if label}<span class="t-label">{label}</span>{:else}<span></span>{/if}
      {#if rightAdornment}
        {@render rightAdornment()}
      {:else if detected}
        <span class="from-exif" aria-hidden="true">● FROM EXIF</span>
        <span class="vh">(pre-filled from EXIF)</span>
      {/if}
    </div>
  {/if}
  {@render children()}
  {#if hint}
    <div class="hint t-meta">{hint}</div>
  {/if}
</label>

<style>
  .field-shell {
    display: block;
    min-width: 0;
  }
  .field-shell__head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 8px;
    gap: 12px;
  }
  .from-exif {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--accent);
    white-space: nowrap;
  }
  .hint {
    margin-top: 6px;
    color: var(--fg-faint);
    font-size: 11px;
  }
  .vh {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
