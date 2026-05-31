<script lang="ts">
  import { cdn } from '$lib/cdn';

  interface Props {
    user: {
      id: string;
      displayName: string;
      handle: string;
      avatarId?: string | null;
      isAdmin?: boolean;
    };
  }

  let { user }: Props = $props();

  // 32px button → request a 64px square so it stays crisp on retina.
  let avatarSrc = $derived(
    user.avatarId ? cdn(user.avatarId, { w: 64, h: 64, fit: 'cover' }) : null
  );

  let open = $state(false);
  let containerEl: HTMLDivElement | undefined = $state();

  function toggle() {
    open = !open;
  }

  function close() {
    open = false;
  }

  // Click-outside + Escape close. The effect cleans up listeners
  // whenever `open` flips back to false (or the component unmounts).
  $effect(() => {
    if (!open) return;
    const onDocClick = (e: MouseEvent) => {
      if (containerEl && !containerEl.contains(e.target as Node)) close();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') close();
    };
    document.addEventListener('click', onDocClick);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('click', onDocClick);
      document.removeEventListener('keydown', onKey);
    };
  });

  let initial = $derived(user.displayName?.[0]?.toUpperCase() ?? 'U');
</script>

<div class="avatar-wrap" bind:this={containerEl}>
  <button type="button" class="avatar" aria-haspopup="menu" aria-expanded={open} onclick={toggle}>
    {#if avatarSrc}
      <img src={avatarSrc} alt={user.displayName} width="32" height="32" />
    {:else}
      {initial}
    {/if}
  </button>

  {#if open}
    <div class="menu" role="menu">
      <div class="menu-greeting">
        <span class="t-meta" style="color: var(--fg-muted);">Signed in as</span>
        <div style="color: var(--fg-primary); font-size: 13px;">{user.displayName}</div>
      </div>
      <div class="menu-divider"></div>
      <a href="/u/{user.handle}" class="menu-item" role="menuitem" onclick={close}>Profile</a>
      <a href="/account/frames" class="menu-item" role="menuitem" onclick={close}>My frames</a>
      <a href="/me/drafts" class="menu-item" role="menuitem" onclick={close}>Drafts</a>
      <a href="/upload" class="menu-item" role="menuitem" onclick={close}>Upload</a>
      <div class="menu-divider"></div>
      {#if user.isAdmin}
        <a href="/admin" class="menu-item menu-item-admin" role="menuitem" onclick={close}>Admin</a>
      {/if}
      <a href="/settings" class="menu-item" role="menuitem" onclick={close}>Settings</a>
      <form method="POST" action="/account/logout">
        <button type="submit" class="menu-item menu-item-button" role="menuitem"> Sign out </button>
      </form>
    </div>
  {/if}
</div>

<style>
  .avatar-wrap {
    position: relative;
  }

  .avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--accent);
    color: var(--accent-ink);
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-display);
    font-size: 15px;
    border: 0;
    cursor: pointer;
    padding: 0;
    overflow: hidden;
  }

  .avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .menu {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    width: 220px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-default);
    border-radius: var(--r-md);
    box-shadow: var(--shadow-md);
    z-index: 100;
    padding: 8px 0;
    animation: menu-in 150ms var(--ease-out);
  }

  @keyframes menu-in {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .menu-greeting {
    padding: 12px 16px;
    line-height: 1.4;
  }

  .menu-divider {
    height: 1px;
    background: var(--border-subtle);
    margin: 4px 0;
  }

  .menu-item,
  .menu-item-button {
    display: block;
    width: 100%;
    padding: 10px 16px;
    text-align: left;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--fg-secondary);
    background: transparent;
    border: 0;
    cursor: pointer;
    text-decoration: none;
  }

  .menu-item:hover,
  .menu-item-button:hover {
    background: var(--bg-raised);
    color: var(--fg-primary);
  }

  .menu-item-admin {
    color: var(--accent);
  }

  form {
    margin: 0;
  }
</style>
