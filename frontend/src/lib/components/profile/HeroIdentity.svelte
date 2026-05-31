<script lang="ts">
  import type { PublicProfile } from '$lib/api/PublicProfile';
  import HeroAvatar from './HeroAvatar.svelte';
  import HeroName from './HeroName.svelte';
  import HeroTagline from './HeroTagline.svelte';
  import HeroSocialLinks from './HeroSocialLinks.svelte';
  import HeroActions from './HeroActions.svelte';

  type EditorSection = 'identity' | 'about' | 'equipment' | 'location' | 'social';

  let {
    profile,
    isOwner,
    hasCover,
    onEditProfile
  }: {
    profile: PublicProfile;
    isOwner: boolean;
    hasCover: boolean;
    onEditProfile: (section?: EditorSection) => void;
  } = $props();
</script>

<section class="identity" class:identity--no-cover={!hasCover}>
  <HeroAvatar
    handle={profile.handle}
    displayName={profile.display_name}
    avatarId={profile.avatar_id}
  />
  <div class="middle">
    <HeroName displayName={profile.display_name} />
    <HeroTagline
      tagline={profile.tagline}
      {isOwner}
      onEditProfile={() => onEditProfile('identity')}
    />
    <HeroSocialLinks links={profile.social_links} />
  </div>
  <HeroActions targetUserId={profile.id} {isOwner} onEditProfile={() => onEditProfile()} />
</section>

<style>
  .identity {
    display: grid;
    grid-template-columns: 144px 1fr auto;
    gap: 24px;
    align-items: start;
    padding: 0 32px 24px;
    margin-top: -80px;
    /* The cover above us is `position: relative` (it needs to anchor the
       "Change cover" / credit chip overlays). That puts the cover into the
       positioned-painting layer, which paints AFTER non-positioned siblings
       even if they come later in the DOM — so without our own positioned
       layer here, the cover image overpaints the upper 80px of the avatar
       that the negative margin pulls into the cover area. */
    position: relative;
    z-index: 1;
  }
  /* Spec line 562: cover is "omitted entirely when empty for visitors".
     Without that banner above, the -80px overlap pulls the avatar into
     the navbar. Drop the negative margin so the avatar lands cleanly
     below the header. */
  .identity.identity--no-cover {
    margin-top: 32px;
  }
  .identity.identity--no-cover .middle {
    padding-top: 0;
  }
  .middle {
    padding-top: 88px;
  }
  @media (max-width: 640px) {
    .identity {
      grid-template-columns: 1fr;
      margin-top: 16px;
      gap: 12px;
    }
    .middle {
      padding-top: 0;
    }
  }
</style>
