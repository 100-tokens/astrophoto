<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { api } from '$lib/api/client';

  interface Props {
    photoId: string;
    initialCount: number;
    initialAppreciated?: boolean;
  }

  let { photoId, initialCount, initialAppreciated = false }: Props = $props();

  // eslint-disable-next-line svelte/valid-compile
  let count = $state(initialCount);
  // eslint-disable-next-line svelte/valid-compile
  let appreciated = $state(initialAppreciated);
  let pending = $state(false);

  async function toggle() {
    if (!page.data.user) {
      await goto(`/signin?return=${encodeURIComponent(page.url.pathname)}`);
      return;
    }
    if (pending) return;
    pending = true;

    const wasOn = appreciated;
    appreciated = !appreciated;
    count += appreciated ? 1 : -1;

    try {
      if (wasOn) await api.appreciations.unappreciate(photoId);
      else await api.appreciations.appreciate(photoId);
    } catch {
      appreciated = wasOn;
      count += wasOn ? 1 : -1;
    } finally {
      pending = false;
    }
  }
</script>

<button
  type="button"
  class="appreciate {appreciated ? 'on' : ''}"
  aria-pressed={appreciated}
  disabled={pending}
  onclick={toggle}
>
  {#if count === 0 && !appreciated}
    ♡ Appreciate
  {:else}
    ♡ {count}
  {/if}
</button>

<style>
  .appreciate {
    background: transparent;
    color: var(--fg-secondary);
    border: 1px solid var(--border-strong);
    padding: 0 12px;
    height: 28px;
    border-radius: 2px;
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: all 150ms ease;
  }
  .appreciate.on {
    color: var(--accent);
    border-color: var(--accent);
    background: rgba(232, 164, 58, 0.06);
  }
  .appreciate:disabled {
    opacity: 0.6;
    cursor: progress;
  }
  .appreciate:hover:not(:disabled) {
    border-color: var(--accent);
  }
</style>
