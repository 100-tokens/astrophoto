<script lang="ts">
  import { untrack } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';

  interface Props {
    open: boolean;
    title: string;
    /** Inline body text above the input. Optional. */
    message?: string;
    /** Placeholder hint inside the input. */
    placeholder?: string;
    /** Pre-seed value when the dialog opens. */
    initialValue?: string;
    /** Confirm-button label. Default "Confirm". */
    confirmLabel?: string;
    /** Cancel-button label. Default "Cancel". */
    cancelLabel?: string;
    /** Input type: "text" or "url". Default "text". */
    type?: 'text' | 'url';
    /** Called with the typed value when the user accepts. */
    onconfirm: (value: string) => void | Promise<void>;
    /** Called when the user cancels. Optional. */
    oncancel?: () => void;
  }

  let {
    open = $bindable(false),
    title,
    message,
    placeholder,
    initialValue = '',
    confirmLabel = 'Confirm',
    cancelLabel = 'Cancel',
    type = 'text',
    onconfirm,
    oncancel
  }: Props = $props();

  let value = $state(untrack(() => initialValue));
  let busy = $state(false);

  // Re-seed value each time the dialog opens — the consumer may want a fresh
  // input each time rather than a persistent draft across opens.
  $effect(() => {
    if (open) value = initialValue;
  });

  async function submit(e: Event) {
    e.preventDefault();
    if (busy) return;
    const trimmed = value.trim();
    if (!trimmed) return;
    busy = true;
    try {
      await onconfirm(trimmed);
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
  {#if message}<p class="dialog-message">{message}</p>{/if}
  <form onsubmit={submit}>
    <input
      class="dialog-input"
      {type}
      {placeholder}
      bind:value
      aria-label={title}
      autocomplete="off"
    />
    <div class="dialog-actions">
      <Button variant="ghost" onclick={cancel} disabled={busy}>{cancelLabel}</Button>
      <Button variant="primary" type="submit" disabled={busy || !value.trim()}>
        {busy ? 'Working…' : confirmLabel}
      </Button>
    </div>
  </form>
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
    margin: 0 0 16px;
    line-height: 1.5;
  }
  .dialog-input {
    width: 100%;
    padding: 10px 12px;
    font-family: var(--font-mono);
    font-size: 13px;
    background: var(--bg-base);
    color: var(--fg-primary);
    border: 1px solid var(--border-default);
    margin-bottom: 24px;
  }
  .dialog-input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .dialog-actions {
    display: flex;
    gap: 12px;
    justify-content: flex-end;
  }
</style>
