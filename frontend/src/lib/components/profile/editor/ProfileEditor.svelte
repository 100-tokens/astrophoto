<script lang="ts">
  import {
    fetchOwnerProfile,
    patchOwnerProfile,
    type ProfilePatchBody
  } from '$lib/api/profileClient';
  import type { Profile } from '$lib/api/Profile';
  import IdentitySection from './IdentitySection.svelte';
  import AvatarSection from './AvatarSection.svelte';
  import AboutSection from './AboutSection.svelte';
  import EquipmentSection from './EquipmentSection.svelte';
  import LocationSection from './LocationSection.svelte';
  import SocialLinksSection from './SocialLinksSection.svelte';

  // Which sections to surface. The full-page "Edit profile" button leaves
  // section unset → all of them. Each inline placeholder (Add the gear, Add
  // a tagline, …) sets section to its own field-group so the user only sees
  // what they actually clicked.
  type Section = 'identity' | 'about' | 'equipment' | 'location' | 'social';

  let {
    open = $bindable<boolean>(false),
    section = $bindable<Section | null>(null),
    onSaved = () => {}
  }: {
    open?: boolean;
    section?: Section | null;
    onSaved?: (profile: Profile) => void;
  } = $props();

  function show(s: Section): boolean {
    return section === null || section === s;
  }

  let titleText = $derived(
    section === 'identity'
      ? 'Edit identity'
      : section === 'about'
        ? 'Tell your story'
        : section === 'equipment'
          ? 'Your gear'
          : section === 'location'
            ? 'Your sky'
            : section === 'social'
              ? 'Where else to find you'
              : 'Edit profile'
  );

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
    // Reset section so the next time the editor opens via the global Edit
    // profile button it shows everything again.
    section = null;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) close();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <div class="overlay" role="dialog" aria-modal="true" aria-label={titleText}>
    <button type="button" class="scrim" aria-label="Close" onclick={close}></button>
    <div class="dialog">
      <header>
        <h2>{titleText}</h2>
        <button type="button" class="x" onclick={close} aria-label="Close">×</button>
      </header>
      <div class="body">
        {#if loading}
          <p class="status">Loading…</p>
        {:else if error}
          <p class="status err">{error}</p>
        {:else if profile}
          {#if show('identity')}
            <AvatarSection
              avatarId={profile.avatar_id}
              displayName={profile.display_name}
              onChanged={(id) => {
                if (profile) profile = { ...profile, avatar_id: id };
              }}
            />
            <IdentitySection
              displayName={profile.display_name}
              tagline={profile.tagline}
              onCommit={async (patch) => {
                await commit(patch);
              }}
            />
          {/if}
          {#if show('about')}
            <AboutSection
              initial={profile.bio_html ?? ''}
              onSave={(html) => commit({ bio_html: html })}
            />
          {/if}
          {#if show('equipment')}
            <EquipmentSection
              equipment={profile.equipment}
              onCommit={async (patch) => {
                await commit(patch);
              }}
            />
          {/if}
          {#if show('location')}
            <LocationSection
              location={profile.location}
              onCommit={async (patch) => {
                await commit(patch);
              }}
            />
          {/if}
          {#if show('social')}
            <SocialLinksSection
              links={profile.social_links}
              onCommit={async (patch) => {
                await commit(patch);
              }}
            />
          {/if}
        {/if}
      </div>
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
    /* The flex column + min-height: 0 dance gives us a sticky header up top
       and an inner .body that scrolls when the viewport is short. Without
       it, on a 600px-tall window the bottom sections get clipped because
       the dialog content grows past the viewport and overflow:auto on the
       outer .dialog couldn't compete with the flex parent's stretching. */
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  header {
    flex: 0 0 auto;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border-subtle);
    padding: 16px 16px 12px;
    background: var(--bg-canvas);
  }
  .body {
    flex: 1 1 auto;
    overflow-y: auto;
    padding: 16px;
    min-height: 0;
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
