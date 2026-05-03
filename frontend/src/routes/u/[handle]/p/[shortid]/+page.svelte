<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import Lightbox from '$lib/components/lightbox/Lightbox.svelte';
  import PhotoDetailFull from '$lib/components/photos/PhotoDetailFull.svelte';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  let asLightbox = $derived((page.state as { lightbox?: boolean } | null)?.lightbox === true);

  function close() {
    history.back();
  }
</script>

{#if asLightbox}
  <Lightbox
    photo={data.photo}
    handle={data.handle}
    morePhotos={data.morePhotos ?? []}
    onClose={close}
    onPrev={data.prevShortid ? () => goto(`/u/${data.handle}/p/${data.prevShortid}`) : undefined}
    onNext={data.nextShortid ? () => goto(`/u/${data.handle}/p/${data.nextShortid}`) : undefined}
  />
{:else}
  <PhotoDetailFull {data} />
{/if}
