<script lang="ts">
  let {
    title,
    description,
    tone = 'default',
    children
  }: {
    title: string;
    description?: string;
    tone?: 'default' | 'danger';
    children: import('svelte').Snippet;
  } = $props();
</script>

<section class="setting-section" data-tone={tone}>
  <h2>{title}</h2>
  {#if description}<p class="desc">{description}</p>{/if}
  <div class="body">{@render children()}</div>
</section>

<style>
  .setting-section {
    border-bottom: 1px solid var(--border-subtle);
    margin-bottom: 40px;
    padding-bottom: 40px;
  }
  h2 {
    font-family: var(--font-display);
    font-size: 26px;
    font-style: italic;
    font-weight: 600;
    margin: 0 0 8px;
    color: var(--fg-primary);
  }
  .setting-section[data-tone='danger'] h2 {
    color: var(--danger);
  }
  .desc {
    font-size: 13px;
    color: var(--fg-muted);
    max-width: 560px;
    margin: 0 0 24px;
  }
</style>
