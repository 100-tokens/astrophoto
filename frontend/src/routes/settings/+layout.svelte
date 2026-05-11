<script lang="ts">
  import { page } from '$app/state';
  import AppHeader from '$lib/components/AppHeader.svelte';
  let { children } = $props();

  const items = [
    { slug: 'profile', label: 'PROFILE', enabled: true, tone: '' },
    { slug: 'equipment', label: 'EQUIPMENT', enabled: false, tone: '' },
    { slug: 'notifications', label: 'NOTIFICATIONS', enabled: false, tone: '' },
    { slug: 'email', label: 'EMAIL & SECURITY', enabled: true, tone: '' },
    { slug: 'appearance', label: 'APPEARANCE', enabled: true, tone: '' },
    { slug: 'sessions', label: 'SESSIONS', enabled: true, tone: '' },
    { slug: 'delete', label: 'DELETE ACCOUNT', enabled: true, tone: 'danger' }
  ];

  let active = $derived(page.url.pathname.split('/').pop() ?? 'profile');

  // Per-section browser tab title. The layout drives this so every sub-page
  // inherits a title without each one repeating <svelte:head>.
  const TITLES: Record<string, string> = {
    profile: 'Profile',
    email: 'Email & Security',
    password: 'Password',
    appearance: 'Appearance',
    sessions: 'Sessions',
    delete: 'Delete account'
  };
  let pageTitle = $derived(`${TITLES[active] ?? 'Settings'} — Astrophoto`);
</script>

<svelte:head>
  <title>{pageTitle}</title>
  <meta name="robots" content="noindex, nofollow" />
</svelte:head>

<AppHeader />

<div class="settings-shell">
  <header class="settings-head">
    <span class="eyebrow">PREFERENCES</span>
    <h1>Account <em>settings</em></h1>
  </header>

  <div class="settings-grid">
    <nav class="settings-nav" aria-label="Settings sections">
      {#each items as item}
        {#if item.enabled}
          <a
            class="nav-item"
            class:active={active === item.slug}
            class:danger={item.tone === 'danger'}
            href="/settings/{item.slug}">{item.label}</a
          >
        {:else}
          <span class="nav-item disabled">
            {item.label}
            <em class="chip chip-soon">SOON</em>
          </span>
        {/if}
      {/each}
      <p class="footer-note">
        ALL CHANGES AUTOSAVE<br />
        EXCEPT EMAIL · PASSWORD<br />
        · DELETION
      </p>
    </nav>
    <main class="settings-content">{@render children()}</main>
  </div>
</div>

<style>
  .settings-shell {
    max-width: 1280px;
    margin: 0 auto;
    padding: 64px;
  }
  .eyebrow {
    font-family: var(--font-mono);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    color: var(--fg-muted);
  }
  h1 {
    font-family: var(--font-display);
    font-size: 48px;
    font-weight: 600;
  }
  .settings-grid {
    display: grid;
    grid-template-columns: 240px 720px;
    gap: 64px;
    margin-top: 32px;
  }
  .settings-nav {
    position: sticky;
    top: 0;
    align-self: start;
  }
  .nav-item {
    display: block;
    padding: 12px 0;
    font-family: var(--font-mono);
    font-size: 12px;
    letter-spacing: 0.12em;
    color: var(--fg-muted);
    text-decoration: none;
  }
  .nav-item.active {
    color: var(--accent);
    border-left: 1px solid var(--accent);
    padding-left: 12px;
    background: var(--bg-accent-tint);
  }
  .nav-item.danger {
    color: var(--danger);
  }
  .nav-item.disabled {
    color: var(--fg-faint);
    cursor: not-allowed;
  }
  .chip-soon {
    font-style: normal;
    margin-left: 8px;
    font-size: 10px;
    padding: 2px 6px;
    border: 1px dashed var(--border-default);
  }
  .footer-note {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--fg-faint);
    margin-top: 32px;
    line-height: 1.6;
  }
</style>
