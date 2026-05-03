<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import Img from '$lib/components/Img.svelte';
  import type { PhotoDetail } from '$lib/api/types';

  interface PageData {
    photo: PhotoDetail;
  }

  let { data }: { data: PageData } = $props();

  let title = $derived(data.photo.target ?? data.photo.original_name);
</script>

<svelte:head>
  <title>{title} — Astrophoto</title>
</svelte:head>

<AppHeader />

<article class="photo-detail">
  <div class="image-wrap">
    <Img photoId={data.photo.id} alt={title} w={1200} sizes="(max-width: 1200px) 100vw, 1200px" />
  </div>
  <div class="info">
    <h1 class="t-display photo-title">{title}</h1>
    {#if data.photo.caption}
      <p class="caption">{data.photo.caption}</p>
    {/if}
  </div>
</article>

<AppFooter />

<style>
  .photo-detail {
    max-width: 1200px;
    margin: 0 auto;
    padding: 32px 32px 64px;
  }

  .image-wrap {
    width: 100%;
    margin-bottom: 24px;
  }

  .image-wrap :global(img) {
    width: 100%;
    height: auto;
    display: block;
  }

  .photo-title {
    font-size: 40px;
    margin: 0 0 16px;
  }

  .caption {
    font-size: 15px;
    line-height: 1.65;
    color: var(--fg-secondary);
    max-width: 720px;
  }

  @media (max-width: 640px) {
    .photo-detail {
      padding: 16px 16px 48px;
    }

    .photo-title {
      font-size: 28px;
    }
  }
</style>
