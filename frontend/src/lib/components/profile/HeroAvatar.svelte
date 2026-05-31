<script lang="ts">
  import { cdn } from '$lib/cdn';

  let {
    handle,
    displayName,
    avatarId = null
  }: {
    handle: string;
    displayName: string;
    avatarId?: string | null;
  } = $props();

  let initial = $derived((displayName[0] ?? handle[0] ?? 'U').toUpperCase());
  // Displayed at 144px (96 on mobile); request 288px for retina, square-cropped.
  let avatarSrc = $derived(avatarId ? cdn(avatarId, { w: 288, h: 288, fit: 'cover' }) : null);
</script>

{#if avatarSrc}
  <img class="avatar" src={avatarSrc} alt={displayName} width="144" height="144" />
{:else}
  <div class="avatar" aria-hidden="true">{initial}</div>
{/if}

<style>
  .avatar {
    width: 144px;
    height: 144px;
    background: var(--accent);
    color: var(--accent-ink);
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-display, 'Source Serif 4', serif);
    font-size: 64px;
    border: 4px solid var(--bg-canvas);
    object-fit: cover;
  }
  @media (max-width: 640px) {
    .avatar {
      width: 96px;
      height: 96px;
      font-size: 44px;
    }
  }
</style>
