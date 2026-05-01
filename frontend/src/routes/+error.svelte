<script lang="ts">
  import { page } from '$app/state';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import MarkReticle from '$lib/components/MarkReticle.svelte';
  import Button from '$lib/components/Button.svelte';

  let status = $derived(page.status);
  let pathname = $derived(page.url?.pathname ?? '/');

  let is404 = $derived(status === 404);

  let eyebrow = $derived(
    is404 ? `● ${status} · NO LIGHT FROM THIS DIRECTION` : `● ${status} · UNEXPECTED ERROR`
  );

  let headline = $derived(
    is404
      ? { before: 'We pointed the scope at ', em: 'nothing', after: '.' }
      : { before: 'Something went ', em: 'wrong', after: ' at our end.' }
  );

  let body = $derived(
    is404
      ? 'The page you asked for is below the horizon — moved, deleted, or it never rose. Try the gallery, or check the address.'
      : 'An unexpected error occurred. Our team has been notified. Try returning to the gallery.'
  );
</script>

<AppHeader />

<div class="error-page bg-grid">
  <div class="error-center">
    <MarkReticle size={88} color="var(--accent)" />

    <div class="t-eyebrow error-eyebrow" style="color: var(--accent);">
      {eyebrow}
    </div>

    <h1 class="error-h1">
      {headline.before}<em>{headline.em}</em>{headline.after}
    </h1>

    <p class="error-body">{body}</p>

    <!-- Technical block -->
    <div class="error-tech">
      <div>REQUESTED · {pathname}</div>
      <div>COORDINATES · UNRESOLVED</div>
    </div>

    <!-- Actions -->
    <div class="error-actions">
      <Button variant="primary" size="lg" href="/">Back to gallery</Button>
      <Button variant="secondary" size="lg" href="/">Search the archive</Button>
    </div>
  </div>
</div>

<AppFooter />

<style>
  .error-page {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: calc(100dvh - 64px - 64px);
    padding: 64px 32px;
  }

  .error-center {
    text-align: center;
    max-width: 540px;
  }

  .error-eyebrow {
    margin-top: 32px;
  }

  .error-h1 {
    font-family: var(--font-display);
    font-size: 56px;
    font-weight: 400;
    margin: 16px 0 0;
    line-height: 1;
  }

  .error-body {
    color: var(--fg-secondary);
    font-size: 15px;
    margin-top: 16px;
  }

  .error-tech {
    margin-top: 32px;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
    letter-spacing: 0.08em;
    background: var(--bg-raised);
    border: 1px solid var(--border-default);
    padding: 16px 20px;
    text-align: left;
    line-height: 1.8;
  }

  .error-actions {
    margin-top: 32px;
    display: flex;
    gap: 12px;
    justify-content: center;
    flex-wrap: wrap;
  }
</style>
