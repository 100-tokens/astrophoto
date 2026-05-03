<script lang="ts">
  import { untrack } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { api } from '$lib/api/client';

  interface Props {
    photoId: string;
    initialCount: number;
    initialAppreciated?: boolean;
    variant?: 'inline' | 'mobile-sticky';
    commentCount?: number;
  }

  let {
    photoId,
    initialCount,
    initialAppreciated = false,
    variant = 'inline',
    commentCount = 0
  }: Props = $props();

  let count = $state(untrack(() => initialCount));
  let appreciated = $state(untrack(() => initialAppreciated));
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

  async function share() {
    if (typeof navigator !== 'undefined' && navigator.share) {
      try {
        await navigator.share({ url: location.href });
      } catch {
        /* user cancelled */
      }
    }
  }
</script>

{#if variant === 'inline'}
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
{:else}
  <div class="mobile-sticky" role="toolbar" aria-label="Photo actions">
    <button
      type="button"
      class="pill"
      class:on={appreciated}
      aria-pressed={appreciated}
      disabled={pending}
      onclick={toggle}
    >
      <span aria-hidden="true">{appreciated ? '♥' : '♡'}</span>
      <span class="num">{count}</span>
    </button>
    <a href="#comments" class="pill"
      ><span aria-hidden="true">💬</span><span class="num">{commentCount}</span></a
    >
    <button type="button" class="pill" onclick={share} aria-label="Share"
      ><span aria-hidden="true">↗</span></button
    >
  </div>
{/if}

<style>
  /* ── Inline variant ───────────────────────────────────────────
     Match the .btn-ghost.btn-sm visual weight of the sibling
     Comments / Share / ⋯ actions in the photo-detail action row.
     A visible border on Appreciate alone made it read as a different
     shape and pulled the eye away from the rest of the row. The
     active (.on) state still uses accent color so the toggle remains
     obvious without making the resting state heavier than its peers. */
  .appreciate {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
    background: transparent;
    color: var(--fg-secondary);
    border: 1px solid transparent;
    padding: 0 12px;
    height: 28px;
    border-radius: 2px;
    font-family: var(--font-ui);
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
    transition:
      color 150ms ease,
      border-color 150ms ease,
      background 150ms ease;
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
    color: var(--accent);
  }

  /* ── Mobile-sticky variant ────────────────────────────────── */
  .mobile-sticky {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    height: 64px;
    background: rgba(20, 20, 20, 0.85);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    border-top: 1px solid var(--border-subtle);
    padding-bottom: env(safe-area-inset-bottom);
    display: flex;
    gap: 12px;
    align-items: center;
    justify-content: space-around;
    z-index: 100;
  }
  .pill {
    height: 44px;
    padding: 0 16px;
    border: 1px solid var(--border-default);
    background: transparent;
    color: var(--fg-primary);
    display: inline-flex;
    align-items: center;
    gap: 8px;
    border-radius: 22px;
    font-family: var(--font-mono);
    font-size: 14px;
    cursor: pointer;
    text-decoration: none;
  }
  .pill.on {
    background: rgba(208, 160, 80, 0.12);
    border-color: var(--accent);
    color: var(--accent);
  }
  .pill:disabled {
    opacity: 0.6;
    cursor: progress;
  }
  .num {
    font-variant-numeric: tabular-nums;
  }
</style>
