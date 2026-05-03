<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import Img from '$lib/components/Img.svelte';

  let { data } = $props();

  let photo = $derived(
    data.photo as {
      id: string;
      target: string | null;
      caption: string | null;
      original_name: string;
    }
  );

  let title = $derived(photo.target ?? photo.original_name);
</script>

<svelte:head>
  <title>{title} — Astrophoto</title>
</svelte:head>

<AppHeader />

<article class="photo-detail">
  <div class="image-wrap">
    <Img photoId={photo.id} alt={title} w={1200} sizes="(max-width: 1200px) 100vw, 1200px" />
  </div>
  <div class="info">
    <h1 class="t-display photo-title">{title}</h1>
    {#if photo.caption}
      <p class="caption">{photo.caption}</p>
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
