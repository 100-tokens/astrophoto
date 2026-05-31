<script lang="ts">
  import { page } from '$app/state';
  import AppHeader from '$lib/components/AppHeader.svelte';

  let { children } = $props();

  const sections = [
    { href: '/admin/equipment', label: 'Equipment' },
    { href: '/admin/settings', label: 'Settings' }
  ];
</script>

<AppHeader />

<div class="admin">
  <aside>
    <p class="eyebrow">Super-admin</p>
    <nav>
      {#each sections as s}
        <a href={s.href} class:active={page.url.pathname.startsWith(s.href)}>{s.label}</a>
      {/each}
    </nav>
  </aside>
  <main>
    {@render children()}
  </main>
</div>

<style>
  .admin {
    display: grid;
    grid-template-columns: 200px 1fr;
    gap: 32px;
    max-width: 1100px;
    margin: 0 auto;
    padding: 32px;
  }
  aside {
    position: sticky;
    top: 32px;
    align-self: start;
  }
  .eyebrow {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--fg-muted);
    margin: 0 0 12px;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  nav a {
    padding: 8px 12px;
    font-family: var(--font-mono);
    font-size: 12px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-secondary);
    text-decoration: none;
    border-left: 2px solid transparent;
  }
  nav a:hover {
    color: var(--fg-primary);
    background: var(--bg-raised);
  }
  nav a.active {
    color: var(--accent);
    border-left-color: var(--accent);
  }
  main {
    min-width: 0;
  }
  @media (max-width: 720px) {
    .admin {
      grid-template-columns: 1fr;
      gap: 16px;
      padding: 16px;
    }
    aside {
      position: static;
    }
    nav {
      flex-direction: row;
      flex-wrap: wrap;
    }
  }
</style>
