<script lang="ts">
  import type { Snippet } from 'svelte';

  // FooterActions — autosave indicator + primary/ghost CTAs. The autosave
  // state is parent-managed (the page derives it from a debounced
  // "dirty since last reset" timer); this component renders only.
  //
  // The CTAs slot is open via `actions` so the page can swap the
  // "draft + continue" pair for "edit caption + save changes" in
  // published-edit mode without this component needing to know about
  // that branch.

  type SaveState = 'idle' | 'saving' | 'saved' | 'error';
  interface Props {
    saveState: SaveState;
    /** ms since last successful save; the parent provides this to keep the
     *  derivation reactive without an internal $effect timer. */
    secondsSinceSaved?: number | null;
    actions: Snippet;
  }

  let { saveState, secondsSinceSaved = null, actions }: Props = $props();

  let dotClass = $derived(
    saveState === 'saving' ? 'dot-saving' : saveState === 'error' ? 'dot-error' : 'dot-saved'
  );

  let label = $derived(() => {
    if (saveState === 'saving') return 'saving…';
    if (saveState === 'error') return 'save failed — retry?';
    if (saveState === 'idle') return 'not yet saved';
    if (secondsSinceSaved == null) return 'auto-saved just now';
    if (secondsSinceSaved < 2) return 'auto-saved just now';
    if (secondsSinceSaved < 60) return `auto-saved ${Math.round(secondsSinceSaved)}s ago`;
    return `auto-saved ${Math.round(secondsSinceSaved / 60)}m ago`;
  });
</script>

<div class="footer">
  <span class="autosave" aria-live="polite">
    <span class={`dot ${dotClass}`} aria-hidden="true"></span>
    <span class="autosave-label">{label()}</span>
  </span>
  <span class="spacer"></span>
  <div class="actions">
    {@render actions()}
  </div>
</div>

<style>
  .footer {
    display: flex;
    align-items: center;
    gap: 16px;
    padding-top: 28px;
    border-top: 1px solid var(--border-subtle);
    flex-wrap: wrap;
  }
  .autosave {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.06em;
    color: var(--fg-muted);
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex: 0 0 6px;
  }
  .dot-saved {
    background: var(--success);
  }
  .dot-saving {
    background: var(--accent);
    animation: dot-pulse 0.9s ease-in-out infinite;
  }
  .dot-error {
    background: var(--danger);
  }
  @keyframes dot-pulse {
    0%,
    100% {
      opacity: 0.4;
    }
    50% {
      opacity: 1;
    }
  }
  .spacer {
    flex: 1;
  }
  .actions {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
  }
  @media (max-width: 640px) {
    .footer {
      flex-direction: column;
      align-items: stretch;
    }
    .spacer {
      display: none;
    }
    .actions {
      flex-direction: column;
    }
    .actions :global(.btn) {
      width: 100%;
    }
  }
</style>
