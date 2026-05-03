<script lang="ts">
  import {
    fetchOwnerProfile,
    patchOwnerProfile,
    type ProfilePatchBody
  } from '$lib/api/profileClient';
  import type { Profile } from '$lib/api/Profile';
  import IdentitySection from './IdentitySection.svelte';
  import AboutSection from './AboutSection.svelte';
  import EquipmentSection from './EquipmentSection.svelte';
  import LocationSection from './LocationSection.svelte';
  import SocialLinksSection from './SocialLinksSection.svelte';

  let {
    open = $bindable<boolean>(false),
    onSaved = () => {}
  }: {
    open?: boolean;
    onSaved?: (profile: Profile) => void;
  } = $props();

  let profile = $state<Profile | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);

  $effect(() => {
    if (open && !profile && !loading) {
      void load();
    }
  });

  async function load() {
    loading = true;
    error = null;
    try {
      profile = await fetchOwnerProfile(fetch);
    } catch (e) {
      error = (e as Error).message;
    } finally {
      loading = false;
    }
  }

  async function commit(patch: ProfilePatchBody) {
    if (!profile) return;
    await patchOwnerProfile(fetch, patch);
    profile = { ...profile, ...patch } as Profile;
    onSaved(profile);
  }

  function close() {
    open = false;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) close();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <div class="overlay" role="dialog" aria-modal="true" aria-label="Edit profile">
    <button type="button" class="scrim" aria-label="Close" onclick={close}></button>
    <div class="dialog">
      <header>
        <h2>Edit profile</h2>
        <button type="button" class="x" onclick={close} aria-label="Close">×</button>
      </header>
      {#if loading}
        <p class="status">Loading…</p>
      {:else if error}
        <p class="status err">{error}</p>
      {:else if profile}
        <IdentitySection
          displayName={profile.display_name}
          tagline={profile.tagline}
          onCommit={async (patch) => {
            await commit(patch);
          }}
        />
        <AboutSection
          initial={profile.bio_html ?? ''}
          onSave={(html) => commit({ bio_html: html })}
        />
        <EquipmentSection
          equipment={profile.equipment}
          onCommit={async (patch) => {
            await commit(patch);
          }}
        />
        <LocationSection
          location={profile.location}
          onCommit={async (patch) => {
            await commit(patch);
          }}
        />
        <SocialLinksSection
          links={profile.social_links}
          onCommit={async (patch) => {
            await commit(patch);
          }}
        />
      {/if}
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: stretch;
    justify-content: flex-end;
  }
  .scrim {
    position: absolute;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    border: 0;
    cursor: default;
  }
  .dialog {
    position: relative;
    width: 480px;
    max-width: 100vw;
    background: var(--bg-canvas);
    border-left: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    overflow-y: auto;
    padding: 16px;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-subtle);
    padding-bottom: 12px;
    margin-bottom: 16px;
  }
  header h2 {
    margin: 0;
    font-family: var(--font-display, serif);
    font-weight: 400;
  }
  .x {
    background: transparent;
    color: var(--fg-muted);
    border: 0;
    font-size: 24px;
    cursor: pointer;
  }
  .status {
    color: var(--fg-muted);
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .status.err {
    color: var(--danger, #c33);
  }
</style>
