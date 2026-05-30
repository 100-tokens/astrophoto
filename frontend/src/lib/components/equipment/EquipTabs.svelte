<script lang="ts">
  type Tab = 'photos' | 'used-with';

  interface Props {
    active: Tab;
    photoCount: number | string;
    usedWithCount: number;
    baseHref: string;
  }

  let { active, photoCount, usedWithCount, baseHref }: Props = $props();

  function fmt(n: number | string): string {
    return Number(n).toLocaleString('en-US');
  }
</script>

<nav class="tabs" aria-label="Equipment view">
  <a
    href={baseHref}
    class="tab"
    class:active={active === 'photos'}
    aria-current={active === 'photos' ? 'page' : undefined}
  >
    Photos<span class="count">{fmt(photoCount)}</span>
  </a>
  <a
    href="{baseHref}?tab=used-with"
    class="tab"
    class:active={active === 'used-with'}
    aria-current={active === 'used-with' ? 'page' : undefined}
  >
    Used with<span class="count">{fmt(usedWithCount)}</span>
  </a>
  <span class="tab tab-disabled">
    Discussion<span class="soon">SOON</span>
  </span>
  <span class="tab tab-disabled">
    History<span class="soon">SOON</span>
  </span>
</nav>

<style>
  .tabs {
    display: flex;
    gap: 32px;
    padding: 0 64px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 16px 0;
    font-family: var(--font-mono);
    font-size: 12px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    text-decoration: none;
    color: var(--fg-muted);
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    transition:
      color 0.15s,
      border-color 0.15s;
  }

  .tab:hover:not(.tab-disabled) {
    color: var(--fg-primary);
  }

  .tab.active {
    color: var(--fg-primary);
    border-bottom-color: var(--accent);
  }

  .tab-disabled {
    color: var(--fg-faint);
    cursor: not-allowed;
  }

  .count {
    font-size: 10px;
    color: var(--fg-faint);
  }

  .tab.active .count {
    color: var(--fg-muted);
  }

  .soon {
    font-size: 9px;
    padding: 2px 6px;
    border: 1px solid var(--border-subtle);
    color: var(--fg-faint);
    letter-spacing: 0.08em;
  }

  @media (max-width: 768px) {
    .tabs {
      padding: 0 16px;
      gap: 20px;
      overflow-x: auto;
      scrollbar-width: none;
    }
    .tabs::-webkit-scrollbar {
      display: none;
    }
    .tab {
      white-space: nowrap;
    }
  }
</style>
