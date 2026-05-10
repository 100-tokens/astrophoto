<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import HeroPage from '$lib/components/profile/HeroPage.svelte';
  import LightboxHost from '$lib/components/discovery/LightboxHost.svelte';
  import ProfileEditor from '$lib/components/profile/editor/ProfileEditor.svelte';
  import CoverPickerModal from '$lib/components/profile/editor/CoverPickerModal.svelte';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  type EditorSection = 'identity' | 'about' | 'equipment' | 'location' | 'social';

  let editorOpen = $state(false);
  let editorSection = $state<EditorSection | null>(null);
  let coverPickerOpen = $state(false);

  let title = $derived(`${data.profile.display_name} — Astrophoto`);
</script>

<svelte:head>
  <title>{title}</title>
</svelte:head>

<AppHeader />

<HeroPage
  profile={data.profile}
  viewMode={data.viewMode}
  firstPage={data.firstPage}
  onEditProfile={(s) => {
    editorSection = s ?? null;
    editorOpen = true;
  }}
  onPickCover={() => (coverPickerOpen = true)}
/>

{#if data.viewMode === 'owner'}
  <ProfileEditor
    bind:open={editorOpen}
    bind:section={editorSection}
    onSaved={() => void invalidateAll()}
  />
  <CoverPickerModal
    bind:open={coverPickerOpen}
    handle={data.profile.handle}
    onPicked={() => void invalidateAll()}
  />
{/if}

<LightboxHost />
<AppFooter />
