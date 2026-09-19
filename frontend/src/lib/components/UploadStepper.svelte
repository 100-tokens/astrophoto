<script lang="ts">
  // Two screens: upload, then verify (equipment, caption, and publish
  // live on that one page). currentStep is 1-based.

  let { currentStep }: { currentStep: 1 | 2 } = $props();

  const STEPS = [
    { id: 1, label: 'UPLOAD' },
    { id: 2, label: 'VERIFY & EQUIP' }
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
      <span class="step-n">{String(step.id).padStart(2, '0')}</span>
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
