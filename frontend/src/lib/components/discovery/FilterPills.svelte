<script lang="ts">
  // FilterPills is a controlled component — it owns no URL or routing state.
  // The parent page passes current values as props and receives changes via callbacks.

  type Sort = 'newest' | 'most-appreciated';
  type Since = '24h' | '7d' | '30d' | 'all';

  let {
    sort = 'newest',
    since = '7d',
    category = undefined,
    following = false,
    variant = 'explore',
    authed = true,
    onSortChange,
    onSinceChange,
    onCategoryChange,
    onFollowingChange,
    onClear
  }: {
    sort?: Sort;
    since?: Since;
    category?: string;
    following?: boolean;
    variant?: 'explore' | 'target' | 'tag' | 'equipment' | 'category' | 'search';
    /** Whether a session is present — the "Following only" toggle is hidden
     * for anonymous visitors (it can only ever return an empty feed). */
    authed?: boolean;
    onSortChange?: (v: Sort) => void;
    onSinceChange?: (v: Since) => void;
    onCategoryChange?: (v: string | undefined) => void;
    onFollowingChange?: (v: boolean) => void;
    onClear?: () => void;
  } = $props();

  const sortOptions: Array<{ label: string; value: Sort }> = [
    { label: 'Newest', value: 'newest' },
    { label: 'Most appreciated', value: 'most-appreciated' }
  ];

  const sinceOptions: Array<{ label: string; value: Since }> = [
    { label: '24 h', value: '24h' },
    { label: '7 d', value: '7d' },
    { label: '30 d', value: '30d' },
    { label: 'All time', value: 'all' }
  ];

  const categoryOptions: Array<{ label: string; value: string }> = [
    { label: 'DSO', value: 'dso' },
    { label: 'Planetary', value: 'planetary' },
    { label: 'Lunar', value: 'lunar' },
    { label: 'Solar', value: 'solar' },
    { label: 'Wide-field', value: 'wide_field' },
    { label: 'Nightscape', value: 'nightscape' }
  ];

  // Show sort pills for all variants; since/category/following only for explore.
  let showSince = $derived(variant === 'explore');
  let showCategory = $derived(variant === 'explore');
  // Following toggle requires a session — anonymous visitors follow nobody, so
  // the backend would return an empty feed with no explanation.
  let showFollowing = $derived(variant === 'explore' && authed);
  let hasActiveFilters = $derived(
    category !== undefined || following || since !== '7d' || sort !== 'newest'
  );
</script>

<section class="filter-rail">
  <div class="left-group">
    {#each sortOptions as opt}
      <button
        type="button"
        class="chip"
        class:chip-accent={sort === opt.value}
        onclick={() => onSortChange?.(opt.value)}
      >
        {opt.label}
      </button>
    {/each}

    {#if showSince}
      <span class="divider" aria-hidden="true"></span>
      {#each sinceOptions as opt}
        <button
          type="button"
          class="chip"
          class:chip-accent={since === opt.value}
          onclick={() => onSinceChange?.(opt.value)}
        >
          {opt.label}
        </button>
      {/each}
    {/if}
  </div>

  {#if showCategory || showFollowing}
    <div class="right-group">
      {#if showCategory}
        {#each categoryOptions as opt}
          <button
            type="button"
            class="chip"
            class:chip-accent={category === opt.value}
            onclick={() => {
              if (category === opt.value) {
                onCategoryChange?.(undefined);
              } else {
                onCategoryChange?.(opt.value);
              }
            }}
          >
            {opt.label}
          </button>
        {/each}
        <span class="divider" aria-hidden="true"></span>
      {/if}

      {#if showFollowing}
        <button
          type="button"
          class="chip"
          class:chip-accent={following}
          onclick={() => onFollowingChange?.(!following)}
        >
          {following ? '✓' : ''} Following only
        </button>
      {/if}

      {#if hasActiveFilters}
        <button
          type="button"
          class="chip chip-clear"
          onclick={() => (onClear ? onClear() : onCategoryChange?.(undefined))}
        >
          ✕ Clear filters
        </button>
      {/if}
    </div>
  {/if}
</section>

<style>
  .filter-rail {
    padding: 20px 64px;
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    flex-wrap: wrap;
  }

  .left-group,
  .right-group {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
  }

  .divider {
    width: 1px;
    height: 20px;
    background: var(--border-default);
    margin: 0 4px;
    flex-shrink: 0;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    border: 1px solid var(--border-default);
    background: transparent;
    color: var(--fg-secondary);
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.04em;
    cursor: pointer;
    transition:
      border-color 0.1s,
      color 0.1s,
      background 0.1s;
  }

  .chip:hover {
    border-color: var(--accent-dim);
    color: var(--fg-primary);
  }

  .chip-accent {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--bg-accent-tint, rgba(180, 150, 60, 0.08));
  }

  .chip-clear {
    /* --fg-muted (~6.5:1 on the dark rail) keeps this de-emphasized vs the
       other chips while meeting WCAG AA; --fg-faint is the disabled token
       and fails contrast for an interactive control. */
    color: var(--fg-muted);
  }
</style>
