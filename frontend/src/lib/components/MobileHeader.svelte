<script lang="ts">
  import Wordmark from './Wordmark.svelte';

  interface Props {
    backHref?: string;
    onMenu?: () => void;
  }

  let { backHref = '/', onMenu }: Props = $props();
</script>

<header class="mobile-header">
  <a href={backHref} aria-label="Back" class="icon-btn">←</a>
  <Wordmark size={16} italic={true} />
  <button type="button" aria-label="More" class="icon-btn" onclick={onMenu}>⋯</button>
</header>

<style>
  .mobile-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 48px;
    padding: 0 16px;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-canvas);
  }

  .icon-btn {
    background: none;
    border: 0;
    color: var(--fg-secondary);
    font-size: 18px;
    line-height: 1;
    padding: 8px;
    cursor: pointer;
  }

  .icon-btn:hover {
    color: var(--fg-primary);
  }
</style>
