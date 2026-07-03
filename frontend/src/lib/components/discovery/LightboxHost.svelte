<script lang="ts">
  import { page } from '$app/state';
  import { preloadData, replaceState } from '$app/navigation';
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

  // Prev/next stay INSIDE the lightbox: shallow-navigate like
  // openLightboxOnClick does (preload the permalink's data, then swap
  // the history entry with the lightbox flag). A plain goto() here was
  // a real navigation — page.state reset, the full detail page mounted,
  // and the feed underneath (scroll position, loaded pages) was gone.
  // replaceState keeps one Back press returning to the feed no matter
  // how far the user stepped through neighbours.
  async function stepTo(handle: string, shortId: string) {
    const url = `/u/${handle}/p/${shortId}`;
    const r = await preloadData(url);
    if (r.type !== 'loaded' || r.status !== 200) {
      window.location.href = url;
      return;
    }
    replaceState(url, { lightbox: true, data: r.data });
  }
</script>

{#if active && stateRef?.data}
  {@const d = stateRef.data}
  <Lightbox
    photo={d.photo}
    handle={d.handle}
    morePhotos={d.morePhotos ?? []}
    onClose={() => history.back()}
    onPrev={d.prevShortid ? () => void stepTo(d.handle, d.prevShortid!) : undefined}
    onNext={d.nextShortid ? () => void stepTo(d.handle, d.nextShortid!) : undefined}
  />
{/if}
