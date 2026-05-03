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

  type ViewMode = 'visitor' | 'owner' | 'admin';

  let {
    profile,
    viewMode = 'visitor',
    onEditProfile = () => {},
    onPickCover = () => {}
  }: {
    profile: PublicProfile;
    viewMode?: ViewMode;
    onEditProfile?: () => void;
    onPickCover?: () => void;
  } = $props();

  let isOwner = $derived(viewMode === 'owner');
  let sort = $state<'newest' | 'popular'>('newest');
</script>

<article class="hero-page" data-mode={viewMode}>
  {#if isOwner}
    <OwnerModeBanner onEdit={onEditProfile} />
  {/if}

  <HeroCover cover={profile.cover} {isOwner} {onPickCover} />

  <HeroIdentity {profile} {isOwner} {onEditProfile} />

  <HeroAbout bio={profile.bio_html} {isOwner} {onEditProfile} />

  <HeroEquipmentStrip equipment={profile.equipment} {isOwner} {onEditProfile} />

  <HeroLocationBadge location={profile.location} {isOwner} {onEditProfile} />

  <HeroStatsRow stats={profile.stats} />

  <FeaturedRow
    items={profile.featured}
    handle={profile.handle}
    {isOwner}
    editorMode={isOwner}
  />

  <GalleryToolbar bind:sort />

  <PhotoGrid handle={profile.handle} {sort} />
</article>

<style>
  .hero-page {
    display: flex;
    flex-direction: column;
    background: var(--bg-canvas);
    color: var(--fg-primary);
  }
</style>
