<script lang="ts">
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';

  interface Props {
    open: boolean;
    title: string;
    /** Body text. Use plain string — no HTML rendering. */
    message: string;
    /** Confirm-button label. Default "Confirm". */
    confirmLabel?: string;
    /** Cancel-button label. Default "Cancel". */
    cancelLabel?: string;
    /** "danger" styles the confirm button red. Default "default". */
    tone?: 'default' | 'danger';
    /** Called when the user accepts. Caller should close the modal. */
    onconfirm: () => void | Promise<void>;
    /** Called when the user cancels or closes (Escape, overlay, X). Optional. */
    oncancel?: () => void;
  }

  let {
    open = $bindable(false),
    title,
    message,
    confirmLabel = 'Confirm',
    cancelLabel = 'Cancel',
    tone = 'default',
    onconfirm,
    oncancel
  }: Props = $props();

  let busy = $state(false);

  async function accept() {
    if (busy) return;
    busy = true;
    try {
      await onconfirm();
    } finally {
      busy = false;
    }
  }

  function cancel() {
    if (busy) return;
    open = false;
    oncancel?.();
  }
</script>

<Modal bind:open {title} {...oncancel !== undefined ? { onclose: oncancel } : {}}>
  <h3 class="dialog-title">{title}</h3>
  <p class="dialog-message">{message}</p>
  <div class="dialog-actions">
    <Button variant="ghost" onclick={cancel} disabled={busy}>{cancelLabel}</Button>
    <Button variant={tone === 'danger' ? 'danger' : 'primary'} onclick={accept} disabled={busy}>
      {busy ? 'Working…' : confirmLabel}
    </Button>
  </div>
</Modal>

<style>
  .dialog-title {
    font-family: var(--font-display);
    font-size: 24px;
    font-weight: 600;
    margin: 0 0 12px;
  }
  .dialog-message {
    color: var(--fg-secondary);
    margin: 0 0 24px;
    line-height: 1.5;
  }
  .dialog-actions {
    display: flex;
    gap: 12px;
    justify-content: flex-end;
  }
</style>
