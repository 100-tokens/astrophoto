<script lang="ts">
  // Three-step stepper for the upload flow. Mirrors the design handoff
  // at docs/design/handoff/screens-2.jsx:119: completed steps carry a ✓,
  // the active step is accent-colored, future steps are muted.
  //
  // currentStep is 1-based: 1 = upload, 2 = verify, 3 = caption.

  let { currentStep }: { currentStep: 1 | 2 | 3 } = $props();

  // Labels match docs/design/handoff-showcase/showcase-p1.jsx:86 — the
  // verify step is "VERIFY EACH" (one frame at a time), and the third
  // step is just "PUBLISH" (caption is part of verify in the handoff).
  const STEPS = [
    { n: '01', label: 'UPLOAD' },
    { n: '02', label: 'VERIFY EACH' },
    { n: '03', label: 'PUBLISH' }
  ];

  function stateFor(idx: number): 'done' | 'active' | 'pending' {
    if (idx + 1 < currentStep) return 'done';
    if (idx + 1 === currentStep) return 'active';
    return 'pending';
  }
</script>

<div class="stepper" role="list" aria-label="Upload progress">
  {#each STEPS as step, i}
    {@const s = stateFor(i)}
    <div class="step step-{s}" role="listitem" aria-current={s === 'active' ? 'step' : undefined}>
      <span class="step-n">{step.n}</span>
      <span>{step.label}</span>
      {#if s === 'done'}
        <span class="check" aria-hidden="true">✓</span>
      {/if}
    </div>
  {/each}
</div>

<style>
  .stepper {
    display: flex;
    margin-top: 32px;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .step {
    flex: 1;
    padding: 16px 0;
    border-top: 2px solid var(--border-default);
    display: flex;
    gap: 12px;
    align-items: center;
    color: var(--fg-muted);
  }
  .step-active,
  .step-done {
    border-top-color: var(--accent);
    color: var(--fg-primary);
  }
  .step-n {
    color: var(--fg-faint);
  }
  .step-active .step-n,
  .step-done .step-n {
    color: var(--accent);
  }
  .check {
    color: var(--accent);
    margin-left: auto;
    margin-right: 32px;
  }
  @media (max-width: 768px) {
    .stepper {
      font-size: 9px;
      margin-top: 20px;
    }
    .step {
      gap: 6px;
    }
    .check {
      margin-right: 12px;
    }
  }
</style>
