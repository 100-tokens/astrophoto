<script lang="ts">
  // VerifyStepper — restyled stepper used by /upload/[id]/verify only.
  // Distinct from UploadStepper (now aligned to the same three-step
  // model) so other routes are not disturbed by the new active-state
  // ● NOW affordance.
  //
  // The handoff defaults to the three-step "Verify & Equip" merged variant
  // because Equipment-as-step-03 overlaps with the page's own Equipment
  // section (UX issue #1). Four-step is supported for product preview.

  type Variant = 'three' | 'four';
  interface Props {
    currentStep: 1 | 2 | 3 | 4;
    variant?: Variant;
  }

  let { currentStep, variant = 'three' }: Props = $props();

  const THREE_STEPS = [
    { id: 1, label: 'UPLOAD' },
    { id: 2, label: 'VERIFY & EQUIP' },
    { id: 3, label: 'CAPTION & PUBLISH' }
  ];
  const FOUR_STEPS = [
    { id: 1, label: 'UPLOAD' },
    { id: 2, label: 'VERIFY DATA' },
    { id: 3, label: 'EQUIPMENT' },
    { id: 4, label: 'CAPTION & PUBLISH' }
  ];

  let steps = $derived(variant === 'four' ? FOUR_STEPS : THREE_STEPS);

  function stateFor(idx: number): 'done' | 'active' | 'pending' {
    if (idx + 1 < currentStep) return 'done';
    if (idx + 1 === currentStep) return 'active';
    return 'pending';
  }
</script>

<div
  class="vstepper"
  role="list"
  aria-label="Upload progress"
  style:grid-template-columns={`repeat(${steps.length}, 1fr)`}
>
  {#each steps as step, i (step.id)}
    {@const s = stateFor(i)}
    <div
      class={`vstep vstep--${s}`}
      role="listitem"
      aria-current={s === 'active' ? 'step' : undefined}
    >
      <span class="vstep-n">{String(step.id).padStart(2, '0')}</span>
      <span class="vstep-label">{step.label}</span>
      {#if s === 'done'}
        <span class="vstep-mark" aria-hidden="true">✓</span>
      {:else if s === 'active'}
        <span class="vstep-mark vstep-now" aria-hidden="true">● NOW</span>
      {/if}
    </div>
  {/each}
</div>

<style>
  .vstepper {
    display: grid;
    gap: 0;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .vstep {
    padding: 14px 0 0;
    border-top: 2px solid var(--border-default);
    color: var(--fg-muted);
    display: flex;
    gap: 12px;
    align-items: center;
  }
  .vstep-n {
    color: var(--fg-faint);
  }
  .vstep--active,
  .vstep--done {
    border-top-color: var(--accent);
    color: var(--fg-primary);
  }
  .vstep--active .vstep-n,
  .vstep--done .vstep-n {
    color: var(--accent);
  }
  .vstep-mark {
    color: var(--accent);
    margin-left: auto;
    margin-right: 16px;
  }
  .vstep-now {
    font-size: 9px;
  }
  @media (max-width: 768px) {
    .vstepper {
      font-size: 9px;
    }
    .vstep {
      gap: 6px;
    }
    .vstep-mark {
      margin-right: 8px;
    }
    /* Three-step on mobile keeps the row; four-step gets too cramped — */
    /* the existing 4-step usage is on /upload, not here. */
  }
</style>
