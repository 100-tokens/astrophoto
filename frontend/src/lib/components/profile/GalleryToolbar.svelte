<script lang="ts">
  type Sort = 'newest' | 'popular';

  let {
    sort = $bindable<Sort>('newest')
  }: {
    sort?: Sort;
  } = $props();
</script>

<header class="toolbar">
  <h2 class="title">Frames</h2>
  <label class="sort">
    <span class="lab">SORT</span>
    <select bind:value={sort}>
      <option value="newest">Newest</option>
      <option value="popular">Popular</option>
    </select>
  </label>
</header>

<style>
  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: 24px 32px 12px;
    border-top: 1px solid var(--border-subtle);
  }
  .title {
    font-family: var(--font-display, 'Source Serif 4', serif);
    font-size: 28px;
    font-weight: 400;
    margin: 0;
  }
  .sort {
    display: flex;
    gap: 8px;
    align-items: center;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-muted);
  }
  .sort select {
    background: transparent;
    color: var(--fg-primary);
    border: 1px solid var(--border-subtle);
    padding: 4px 8px;
    font-family: var(--font-mono);
    font-size: 12px;
  }
</style>
