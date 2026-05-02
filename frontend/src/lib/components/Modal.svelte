<script lang="ts">
  let {
    open = $bindable(false),
    title,
    children,
    onclose
  }: {
    open: boolean;
    title: string;
    children: import('svelte').Snippet;
    onclose?: () => void;
  } = $props();

  let dialogEl: HTMLDivElement = $state() as HTMLDivElement;
  let invokerBefore: HTMLElement | null = null;

  $effect(() => {
    if (open) {
      invokerBefore = document.activeElement as HTMLElement | null;
      // Focus the first focusable inside the dialog after the DOM updates.
      queueMicrotask(() => {
        const focusable = dialogEl?.querySelector<HTMLElement>(
          'a, button, input, textarea, select, [tabindex]:not([tabindex="-1"])'
        );
        focusable?.focus();
      });
    } else if (invokerBefore) {
      invokerBefore.focus();
    }
  });

  function onKey(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      open = false;
      onclose?.();
    }
    if (e.key === 'Tab' && dialogEl) {
      // Trap focus.
      const focusables = dialogEl.querySelectorAll<HTMLElement>(
        'a, button, input, textarea, select, [tabindex]:not([tabindex="-1"])'
      );
      if (!focusables.length) return;
      const first = focusables[0] as HTMLElement | undefined;
      const last = focusables[focusables.length - 1] as HTMLElement | undefined;
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last?.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first?.focus();
      }
    }
  }

  $effect(() => {
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="modal-overlay"
    onclick={() => {
      open = false;
      onclose?.();
    }}
  ></div>
  <div
    class="modal-dialog"
    role="dialog"
    aria-modal="true"
    aria-label={title}
    tabindex="-1"
    bind:this={dialogEl}
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    {@render children()}
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: var(--bg-overlay);
    z-index: 50;
  }
  .modal-dialog {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    background: var(--bg-raised);
    border: 1px solid var(--border-default);
    padding: 32px;
    max-width: 640px;
    width: 90vw;
    z-index: 51;
    border-radius: var(--r-md, 4px);
  }
</style>
