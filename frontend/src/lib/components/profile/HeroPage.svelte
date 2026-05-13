<script lang="ts">
  import type { PublicProfile } from '$lib/api/PublicProfile';
  import OwnerModeBanner from './OwnerModeBanner.svelte';
  import HeroCover from './HeroCover.svelte';
  import HeroIdentity from './HeroIdentity.svelte';
  import HeroAbout from './HeroAbout.svelte';
  import HeroEquipmentStrip from './HeroEquipmentStrip.svelte';
  import HeroLocationBadge from './HeroLocationBadge.svelte';
  import HeroStatsRow from './HeroStatsRow.svelte';
  import FeaturedRow from './FeaturedRow.svelte';
  import GalleryToolbar from './GalleryToolbar.svelte';
  import PhotoGrid from './PhotoGrid.svelte';
  import type { GalleryPhoto } from '$lib/api/GalleryPhoto';

  type ViewMode = 'visitor' | 'owner' | 'admin';
  type FirstPage = { photos: GalleryPhoto[]; next_cursor: string | null };

  // The route owns editor open/section state; we surface the click as
  // (section?) so the route can decide whether to open the full sheet
  // (no arg from the global Edit profile button) or a single-section sheet
  // (arg from an inline placeholder click).
  type EditorSection = 'identity' | 'about' | 'equipment' | 'location' | 'social';

  let {
    profile,
    viewMode = 'visitor',
    firstPage = null,
    onEditProfile = () => {},
    onPickCover = () => {}
  }: {
    profile: PublicProfile;
    viewMode?: ViewMode;
    firstPage?: FirstPage | null;
    onEditProfile?: (section?: EditorSection) => void;
    onPickCover?: () => void;
  } = $props();

  let isOwner = $derived(viewMode === 'owner');
  let sort = $state<'newest' | 'popular'>('newest');
</script>

<article class="hero-page" data-mode={viewMode}>
  {#if isOwner}
    <OwnerModeBanner onEdit={() => onEditProfile()} />
  {/if}

  <HeroCover cover={profile.cover} {isOwner} {onPickCover} />

  <HeroIdentity
    {profile}
    {isOwner}
    onEditProfile={(s) => onEditProfile(s)}
    hasCover={profile.cover !== null && profile.cover !== undefined}
  />

  <HeroAbout bio={profile.bio_html} {isOwner} onEditProfile={() => onEditProfile('about')} />

  <HeroEquipmentStrip
    equipment={profile.equipment}
    {isOwner}
    onEditProfile={() => onEditProfile('equipment')}
  />

  <HeroLocationBadge
    location={profile.location}
    {isOwner}
    onEditProfile={() => onEditProfile('location')}
  />

  <HeroStatsRow stats={profile.stats} />

  <FeaturedRow items={profile.featured} handle={profile.handle} {isOwner} editorMode={isOwner} />

  <GalleryToolbar bind:sort />

  <PhotoGrid handle={profile.handle} {sort} initial={firstPage} />
</article>

<style>
  .hero-page {
    display: flex;
    flex-direction: column;
    background: var(--bg-canvas);
    color: var(--fg-primary);
  }
</style>
