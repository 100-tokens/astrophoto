<script lang="ts">
  // Hamburger menu for the primary nav on viewports under 768px, where
  // `.primary-nav` is hidden. The same destinations (Gallery / Targets /
  // Photographers / About) were otherwise unreachable on mobile — the
  // header nav was hidden with no working fallback (the footer only carries
  // About/Terms/Privacy/…). Mirrors AvatarMenu's open/close + styling so the
  // two header menus feel like one system.

  interface Props {
    links: Array<{ label: string; href: string }>;
    active?: string;
  }

  let { links, active }: Props = $props();

  let open = $state(false);
  let containerEl: HTMLDivElement | undefined = $state();

  function toggle() {
    open = !open;
  }
  function close() {
    open = false;
  }

  // Click-outside + Escape close (same pattern as AvatarMenu). Cleaned up
  // whenever `open` flips false or the component unmounts.
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
</script>

<div class="mobile-nav" bind:this={containerEl}>
  <button
    type="button"
    class="burger"
    class:open
    aria-label={open ? 'Close menu' : 'Open menu'}
    aria-expanded={open}
    onclick={toggle}
  >
    <span></span>
    <span></span>
    <span></span>
  </button>

  {#if open}
    <nav class="menu" aria-label="Main navigation">
      {#each links as link}
        <a
          href={link.href}
          class="menu-item"
          class:active={active === link.label}
          aria-current={active === link.label ? 'page' : undefined}
          onclick={close}
        >
          {link.label}
        </a>
      {/each}
    </nav>
  {/if}
</div>

<style>
  /* Hidden on desktop — the inline .primary-nav covers that breakpoint. */
  .mobile-nav {
    position: relative;
    display: none;
  }
  @media (max-width: 768px) {
    .mobile-nav {
      display: block;
    }
  }

  .burger {
    width: 32px;
    height: 32px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 5px;
    padding: 7px;
    background: transparent;
    border: 0;
    cursor: pointer;
  }
  .burger span {
    display: block;
    height: 1.5px;
    width: 100%;
    background: var(--fg-secondary);
    transition:
      transform 150ms var(--ease-out),
      opacity 120ms,
      background 120ms;
  }
  .burger:hover span {
    background: var(--accent);
  }
  /* Morph into an ✕ when open. */
  .burger.open span:nth-child(1) {
    transform: translateY(6.5px) rotate(45deg);
    background: var(--accent);
  }
  .burger.open span:nth-child(2) {
    opacity: 0;
  }
  .burger.open span:nth-child(3) {
    transform: translateY(-6.5px) rotate(-45deg);
    background: var(--accent);
  }

  /* Dropdown — matches AvatarMenu.menu exactly. */
  .menu {
    position: absolute;
    top: calc(100% + 8px);
    left: 0;
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
  .menu-item {
    display: block;
    width: 100%;
    padding: 10px 16px;
    text-align: left;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--fg-secondary);
    text-decoration: none;
  }
  .menu-item:hover {
    background: var(--bg-raised);
    color: var(--fg-primary);
  }
  .menu-item.active {
    color: var(--fg-primary);
  }
  .menu-item.active::before {
    content: '● ';
    color: var(--accent);
  }
</style>
