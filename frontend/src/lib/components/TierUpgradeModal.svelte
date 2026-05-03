<script lang="ts">
  import Modal from './Modal.svelte';

  interface Props {
    open?: boolean;
    onClose?: () => void;
  }

  let { open = $bindable(false), onClose }: Props = $props();
</script>

<Modal bind:open title="Upgrade tier" {...onClose ? { onclose: onClose } : {}}>
  <p class="t-eyebrow">● UPGRADE · UPLOAD LARGER MASTERS</p>
  <h2 class="t-display t-display-i">Need <em>more headroom</em>?</h2>
  <p>Subscribers can upload 16-bit TIFFs up to <strong>200 MB</strong>.</p>

  <div class="tu-tiers">
    <div class="tu-tier">
      <h3>Free</h3>
      <p class="t-meta">50 MB / file</p>
      <p>Stays free, always.</p>
    </div>
    <div class="tu-tier tu-tier-recommended">
      <span class="chip chip-accent tu-chip">RECOMMENDED</span>
      <h3>Subscriber</h3>
      <p class="t-meta">200 MB / file</p>
      <p>Coming soon.</p>
    </div>
  </div>

  <div class="tu-actions">
    <button
      class="btn btn-ghost"
      onclick={() => {
        open = false;
        onClose?.();
      }}
    >
      Maybe later
    </button>
    <button class="btn btn-primary" disabled aria-disabled="true"> Upgrade — coming soon </button>
  </div>
</Modal>

<style>
  .tu-tiers {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    margin: 24px 0;
  }

  .tu-tier {
    padding: 20px;
    border: 1px solid var(--border-default);
    border-radius: var(--r-md, 4px);
  }

  .tu-tier-recommended {
    border-color: var(--accent-dim);
    background: color-mix(in oklab, var(--accent) 10%, transparent);
    position: relative;
  }

  .tu-chip {
    position: absolute;
    top: 12px;
    right: 12px;
  }

  .tu-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
</style>
