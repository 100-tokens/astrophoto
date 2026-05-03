<script lang="ts">
  let {
    active,
    counts,
    sort,
    view
  }: {
    active: 'all' | 'published' | 'drafts';
    counts: { all: number; published: number; drafts: number };
    sort: 'newest' | 'oldest';
    view: 'list' | 'grid';
  } = $props();

  function href(filter: string) {
    const p = new URLSearchParams({ filter, sort, view });
    return `/account/frames?${p.toString()}`;
  }
  function sortHref(s: string) {
    const p = new URLSearchParams({ filter: active, sort: s, view });
    return `/account/frames?${p.toString()}`;
  }
  function viewHref(v: string) {
    const p = new URLSearchParams({ filter: active, sort, view: v });
    return `/account/frames?${p.toString()}`;
  }
</script>

<div class="filter-bar">
  <div class="chips">
    <a href={href('all')} class:on={active === 'all'}>All · {counts.all}</a>
    <a href={href('published')} class:on={active === 'published'}>Published · {counts.published}</a>
    <a href={href('drafts')} class:on={active === 'drafts'}>Drafts · {counts.drafts}</a>
  </div>
  <div class="controls">
    <a href={sortHref(sort === 'newest' ? 'oldest' : 'newest')} class="t-meta">
      SORT: {sort.toUpperCase()}
    </a>
    <a href={viewHref(view === 'list' ? 'grid' : 'list')} class="t-meta">
      VIEW: {view.toUpperCase()}
    </a>
  </div>
</div>

<style>
  .filter-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 0;
    border-bottom: 1px solid var(--border-subtle);
  }
  .chips {
    display: flex;
    gap: 12px;
  }
  .chips a {
    padding: 6px 12px;
    border: 1px solid var(--border-default);
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.08em;
    color: var(--fg-secondary);
    text-decoration: none;
  }
  .chips a.on {
    color: var(--fg-primary);
    border-color: var(--accent);
  }
  .controls {
    display: flex;
    gap: 16px;
  }
</style>
