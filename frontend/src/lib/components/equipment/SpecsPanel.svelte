<script lang="ts">
  import type { Snippet } from 'svelte';

  type Props = {
    mode?: 'create' | 'edit';
    footerNote?: string;
    onSave?: () => void;
    onDiscard?: () => void;
    children?: Snippet;
  };

  let { mode = 'edit', footerNote, onSave, onDiscard, children }: Props = $props();

  const accentColor = $derived(mode === 'create' ? 'var(--accent)' : 'var(--warning)');
  const labelText = $derived(
    mode === 'create' ? 'NEW · WILL JOIN THE SHARED CATALOG' : 'EDITING A SHARED CATALOG ITEM'
  );
</script>

<div class="specs-panel" style="--accent-col: {accentColor}">
  <div class="specs-panel-head">
    <span class="t-label" style="color: var(--accent-col)">● {labelText}</span>
    <span class="spacer"></span>
    <span class="t-meta">SPECS HELP OTHERS FIND YOUR FRAMES</span>
  </div>
  <div class="specs-panel-body">
    {@render children?.()}
  </div>
  {#if footerNote || onSave}
    <div class="specs-panel-footer">
      {#if footerNote}
        <span class="t-meta footer-note">{footerNote}</span>
      {/if}
      <button type="button" class="btn btn-ghost btn-sm" onclick={() => onDiscard?.()}>
        Discard
      </button>
      <button type="button" class="btn btn-primary btn-sm" onclick={() => onSave?.()}>
        Save to catalog
      </button>
    </div>
  {/if}
</div>

<style>
  .specs-panel {
    border: 1px solid var(--border-default);
    border-left: 2px solid var(--accent-col);
    background: var(--bg-base);
  }

  .specs-panel-head {
    padding: 10px 16px;
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .spacer {
    flex: 1;
  }

  .specs-panel-body {
    padding: 20px;
  }

  .specs-panel-footer {
    padding: 12px 16px;
    border-top: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .footer-note {
    flex: 1;
  }
</style>
