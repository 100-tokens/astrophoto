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
  <em class="untitled-chip">UNTITLED</em>
{:else}
  <em class="untitled-chip">UNTITLED</em>
{/if}

<style>
  .title { color: var(--fg-primary); }
  .title.size-sm { font-size: 14px; }
  .title.size-md { font-size: 18px; }
  .title.size-lg { font-family: var(--font-display); font-size: 36px; }
  .untitled { font-style: italic; color: var(--fg-secondary); }
  .untitled-chip {
    display: inline-block;
    margin-left: 8px;
    padding: 2px 6px;
    border: 1px dashed var(--border-default);
    border-radius: 2px;
    font-family: var(--font-mono);
    font-size: 9px;
    letter-spacing: 0.1em;
    color: var(--fg-muted);
    text-transform: uppercase;
    font-style: normal;
    vertical-align: middle;
  }
</style>
