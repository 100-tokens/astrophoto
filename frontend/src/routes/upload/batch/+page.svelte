<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import Button from '$lib/components/Button.svelte';
  import TargetPicker from '$lib/components/TargetPicker.svelte';
  import TagInput from '$lib/components/TagInput.svelte';
  import Img from '$lib/components/Img.svelte';
  import UploadStepper from '$lib/components/UploadStepper.svelte';
  import type { PageProps } from './$types';

  let { data, form }: PageProps = $props();
  let target = $state('');
  let tags = $state<string[]>([]);
</script>

<svelte:head><title>Apply to all — Astrophoto</title></svelte:head>
<AppHeader active="Gallery" />

<main>
  <div class="page">
    <div class="t-eyebrow">NEW FRAMES</div>
    <h1 class="title">Apply to <em>all</em>.</h1>
    <UploadStepper currentStep={2} />

    <p class="lede">
      Set fields shared by all {data.photos.length} frames. You can override per-photo on the next step.
    </p>

    <form method="POST" class="form">
      <input type="hidden" name="ids" value={data.ids.join(',')} />

      <div class="field">
        <TargetPicker bind:value={target} />
      </div>

      <div class="field">
        <TagInput bind:value={tags} />
      </div>

      <div class="thumb-strip">
        {#each data.photos as photo}
          <div class="thumb"><Img photoId={photo.id} w={144} alt={photo.original_name} /></div>
        {/each}
      </div>

      {#if form?.error}<p class="err">{form.error}</p>{/if}

      <div class="actions">
        <Button variant="ghost" href={`/upload/batch/edit?ids=${data.ids.join(',')}`}>Skip</Button>
        <Button variant="primary" type="submit">Continue →</Button>
      </div>
    </form>
  </div>
</main>

<style>
  .page {
    max-width: 720px;
    margin: 0 auto;
    padding: 40px 64px 64px;
  }
  .title {
    font-family: var(--font-display);
    font-size: 44px;
    margin: 8px 0 12px;
  }
  .lede {
    color: var(--fg-secondary);
    margin: 24px 0 32px;
    max-width: 64ch;
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 24px;
  }
  .thumb-strip {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .thumb {
    width: 72px;
    height: 72px;
    overflow: hidden;
    background: var(--bg-elevated);
  }
  .thumb :global(img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    margin-top: 24px;
  }
  .err {
    color: var(--danger);
  }
  @media (max-width: 768px) {
    .page {
      padding: 32px 24px;
    }
  }
</style>
