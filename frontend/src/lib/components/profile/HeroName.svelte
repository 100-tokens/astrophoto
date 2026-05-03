<script lang="ts">
  let { displayName }: { displayName: string } = $props();

  let parts = $derived(displayName.trim().split(/\s+/));
  let first = $derived(parts[0] ?? displayName);
  let rest = $derived(parts.slice(1).join(' '));
</script>

<h1 class="name">
  {first}{#if rest} <em>{rest}</em>{/if}
</h1>

<style>
  .name {
    font-family: var(--font-display, 'Source Serif 4', serif);
    font-weight: 400;
    font-size: 64px;
    line-height: 1;
    margin: 0;
  }
  .name em {
    font-style: italic;
  }
  @media (max-width: 640px) {
    .name {
      font-size: 40px;
    }
  }
</style>
