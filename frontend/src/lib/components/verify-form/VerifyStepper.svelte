<script lang="ts">
  // Verify page stepper. Caption and publish are on this same screen, so
  // the flow is two steps, not three.

  interface Props {
    currentStep: 1 | 2;
  }

  let { currentStep }: Props = $props();

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

<div class="vstepper" role="list" aria-label="Upload progress">
  {#each STEPS as step, i (step.id)}
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
    grid-template-columns: repeat(2, 1fr);
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
  }
</style>
