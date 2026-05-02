<script lang="ts">
  import { untrack } from 'svelte';
  import { goto, invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import { api } from '$lib/api/client';

  interface Props {
    userId: string;
    initialFollowing: boolean;
  }

  let { userId, initialFollowing }: Props = $props();

  let following = $state(untrack(() => initialFollowing));
  let pending = $state(false);
  let hovering = $state(false);

  async function toggle() {
    if (!page.data.user) {
      await goto(`/signin?return=${encodeURIComponent(page.url.pathname)}`);
      return;
    }
    if (pending) return;
    pending = true;

    const wasOn = following;
    following = !following;

    try {
      if (wasOn) await api.follows.unfollow(userId);
      else await api.follows.follow(userId);
      // Refresh layout data so /me's following_ids updates and any
      // dependent view (like the gallery feed) recomputes.
      await invalidateAll();
    } catch {
      following = wasOn;
    } finally {
      pending = false;
      hovering = false;
    }
  }

  let label = $derived(following ? (hovering ? 'Unfollow' : '✓ Following') : 'Follow');
</script>

<button
  type="button"
  class="follow {following ? 'on' : ''} {following && hovering ? 'hover-off' : ''}"
  disabled={pending}
  onclick={toggle}
  onmouseenter={() => (hovering = true)}
  onmouseleave={() => (hovering = false)}
>
  {label}
</button>

<style>
  .follow {
    background: var(--accent);
    color: var(--accent-ink);
    border: 1px solid var(--accent);
    padding: 0 16px;
    height: 36px;
    border-radius: 2px;
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: all 150ms ease;
  }
  .follow.on {
    background: transparent;
    color: var(--accent);
  }
  .follow.on.hover-off {
    color: var(--danger);
    border-color: var(--danger);
  }
  .follow:disabled {
    opacity: 0.6;
    cursor: progress;
  }
</style>
