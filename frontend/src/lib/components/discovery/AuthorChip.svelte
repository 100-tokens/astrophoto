<script lang="ts">
  let { handle }: { handle: string } = $props();
</script>

<a class="author-chip" href="/u/{handle}" onclick={(e) => e.stopPropagation()}>
  @{handle.toUpperCase()}
</a>

<style>
  .author-chip {
    display: inline-block;
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.06em;
    color: rgba(255, 255, 255, 0.65);
    text-decoration: none;
    line-height: 1;
  }

  .author-chip:hover {
    color: rgba(255, 255, 255, 0.9);
    text-decoration: underline;
  }
</style>
