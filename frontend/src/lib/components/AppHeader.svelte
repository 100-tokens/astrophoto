<script lang="ts">
  import { page } from '$app/state';
  import MarkReticle from './MarkReticle.svelte';
  import Wordmark from './Wordmark.svelte';
  import AvatarMenu from './AvatarMenu.svelte';
  import SearchBar from './discovery/SearchBar.svelte';
  import { cls } from '$lib/utils/cls';

  interface Props {
    active?: 'Gallery' | 'Targets' | 'Photographers' | 'About';
    class?: string;
  }

  let { active = 'Gallery', class: className }: Props = $props();

  // Auth state comes from layout data resolved by hooks.server.ts.
  let user = $derived(page.data.user);

  const navLinks: Array<{
    label: 'Gallery' | 'Targets' | 'Photographers' | 'About';
    href: string;
  }> = [
    { label: 'Gallery', href: '/' },
    { label: 'Targets', href: '/t' },
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
    <!-- Search bar (⌘K focusable, debounced autocomplete) -->
    <SearchBar />

    {#if user}
      <a href="/upload" class="btn btn-secondary btn-sm">Upload</a>
      <AvatarMenu {user} />
    {:else}
      <a href="/signin" class="nav-link">Sign in</a>
      <a href="/signup" class="btn btn-primary btn-sm">Create account</a>
    {/if}
  </div>
</header>
