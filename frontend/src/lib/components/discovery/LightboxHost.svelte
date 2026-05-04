<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import Lightbox from '$lib/components/lightbox/Lightbox.svelte';
  import type { PhotoDetail } from '$lib/api/types';
  import type { GalleryPhoto } from '$lib/api/GalleryPhoto';

  type LightboxData = {
    photo: PhotoDetail;
    handle: string;
    morePhotos?: GalleryPhoto[];
    prevShortid?: string | null;
    nextShortid?: string | null;
  };
  type LightboxState = { lightbox?: boolean; data?: LightboxData } | null;

  let stateRef = $derived(page.state as LightboxState);
  let active = $derived(stateRef?.lightbox === true && !!stateRef?.data);
</script>

{#if active && stateRef?.data}
  {@const d = stateRef.data}
  <Lightbox
    photo={d.photo}
    handle={d.handle}
    morePhotos={d.morePhotos ?? []}
    onClose={() => history.back()}
    onPrev={d.prevShortid ? () => goto(`/u/${d.handle}/p/${d.prevShortid}`) : undefined}
    onNext={d.nextShortid ? () => goto(`/u/${d.handle}/p/${d.nextShortid}`) : undefined}
  />
{/if}
