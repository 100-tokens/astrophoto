<script lang="ts">
  // Reusable empty / no-results state. Editorial dark theme, on-brand
  // reticle motif. Use across list routes (gallery, targets, tags,
  // profiles, equipment, search, drafts) for a consistent SOTA feel.
  import MarkReticle from '$lib/components/MarkReticle.svelte';

  let {
    title,
    message,
    ctaLabel,
    ctaHref,
    compact = false
  }: {
    title: string;
    message?: string;
    ctaLabel?: string;
    ctaHref?: string;
    compact?: boolean;
  } = $props();
</script>

<div class="empty-state" class:compact role="status">
  <span class="empty-mark" aria-hidden="true"><MarkReticle size={compact ? 36 : 52} /></span>
  <p class="empty-title">{title}</p>
  {#if message}<p class="empty-message">{message}</p>{/if}
  {#if ctaLabel && ctaHref}
    <a class="empty-cta" href={ctaHref}>{ctaLabel} <span aria-hidden="true">→</span></a>
  {/if}
</div>

<style>
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 0.7rem;
    padding: 4.5rem 1.5rem;
  }
  .empty-state.compact {
    padding: 2.5rem 1rem;
    gap: 0.5rem;
  }
  .empty-mark {
    color: var(--fg-faint);
    opacity: 0.65;
    margin-bottom: 0.4rem;
  }
  .empty-title {
    font-family: var(--font-display);
    font-size: 1.3rem;
    line-height: 1.2;
    color: var(--fg-primary);
    margin: 0;
  }
  .compact .empty-title {
    font-size: 1.1rem;
  }
  .empty-message {
    font-size: 0.9rem;
    color: var(--fg-muted);
    max-width: 36ch;
    line-height: 1.55;
    margin: 0;
  }
  .empty-cta {
    margin-top: 0.6rem;
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.55rem 1.15rem;
    border: 1px solid var(--border-default);
    border-radius: var(--r-md);
    color: var(--fg-primary);
    font-size: 0.85rem;
    letter-spacing: 0.01em;
    text-decoration: none;
    transition:
      border-color 150ms var(--ease-out, ease),
      background 150ms var(--ease-out, ease),
      color 150ms var(--ease-out, ease);
  }
  .empty-cta:hover {
    border-color: var(--accent);
    background: var(--bg-elevated);
  }
</style>
