<script lang="ts">
  let {
    photo,
    size = 'md'
  }: {
    photo: { target?: string | null; original_name?: string | null };
    size?: 'sm' | 'md' | 'lg';
  } = $props();
</script>

{#if photo.target}
  <span class="title size-{size}">{photo.target}</span>
{:else if photo.original_name}
  <span class="title untitled size-{size}">{photo.original_name}</span>
{:else}
  <span class="title untitled size-{size}">Untitled</span>
{/if}

<style>
  .title {
    color: var(--fg-primary);
  }
  .title.size-sm {
    font-size: 14px;
  }
  .title.size-md {
    font-size: 18px;
  }
  .title.size-lg {
    font-family: var(--font-display);
    font-size: 36px;
  }
  .untitled {
    font-style: italic;
    color: var(--fg-secondary);
  }
</style>
