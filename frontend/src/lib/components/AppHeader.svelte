<script lang="ts">
  import MarkReticle from './MarkReticle.svelte';
  import Wordmark from './Wordmark.svelte';
  import { cls } from '$lib/utils/cls';

  interface Props {
    active?: 'Gallery' | 'Targets' | 'Photographers' | 'About';
    auth?: boolean;
    userInitial?: string;
    class?: string;
  }

  let { active = 'Gallery', auth = false, userInitial = 'M', class: className }: Props = $props();

  const navLinks: Array<{
    label: 'Gallery' | 'Targets' | 'Photographers' | 'About';
    href: string;
  }> = [
    { label: 'Gallery', href: '/' },
    { label: 'Targets', href: '/targets' },
    { label: 'Photographers', href: '/photographers' },
    { label: 'About', href: '/about' }
  ];
</script>

<header class={cls('app-header', className)}>
  <!-- Logo -->
  <div style="display: flex; align-items: center; gap: 12px;">
    <MarkReticle size={28} color="var(--accent)" />
    <Wordmark size={22} weight={600} italic={false} />
  </div>

  <!-- Center nav -->
  <nav style="display: flex; gap: 32px;" aria-label="Main navigation">
    {#each navLinks as link}
      <a
        href={link.href}
        class={cls('nav-link', active === link.label && 'active')}
        aria-current={active === link.label ? 'page' : undefined}
      >
        {link.label}
      </a>
    {/each}
  </nav>

  <!-- Right: search + auth -->
  <div style="display: flex; align-items: center; gap: 12px;">
    <!-- Search field -->
    <div
      style="display: flex; align-items: center; gap: 8px; padding: 0 12px; height: 32px; border: 1px solid var(--border-default); border-radius: 2px; color: var(--fg-muted); font-family: var(--font-mono); font-size: 12px; width: 220px;"
      role="search"
    >
      <!-- Magnifying glass icon (inline SVG, no external dep) -->
      <svg
        width="12"
        height="12"
        viewBox="0 0 16 16"
        fill="none"
        stroke="currentColor"
        stroke-width="1.2"
        aria-hidden="true"
      >
        <circle cx="7" cy="7" r="5" />
        <line x1="11" y1="11" x2="14" y2="14" />
      </svg>
      <span>search the archive…</span>
      <span style="margin-left: auto; font-size: 10px; letter-spacing: 0.1em;">⌘K</span>
    </div>

    {#if auth}
      <a href="/upload" class="btn btn-secondary btn-sm">Upload</a>
      <div
        style="width: 32px; height: 32px; border-radius: 50%; background: var(--accent); color: var(--accent-ink); display: flex; align-items: center; justify-content: center; font-family: var(--font-display); font-size: 15px;"
        aria-label="User menu"
      >
        {userInitial}
      </div>
    {:else}
      <a href="/signin" class="nav-link">Sign in</a>
      <a href="/signup" class="btn btn-primary btn-sm">Create account</a>
    {/if}
  </div>
</header>
