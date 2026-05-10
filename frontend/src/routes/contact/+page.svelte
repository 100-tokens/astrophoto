<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
</script>

<svelte:head><title>Contact — Astrophoto</title></svelte:head>

<AppHeader />

<main class="static-page">
  <div class="t-eyebrow accent">● CONTACT</div>
  <h1>Reach out.</h1>

  <p>
    Bug, feature request, or just want to say hello? Email us at
    <a href="mailto:hello@astrophoto.example">hello@astrophoto.example</a>.
  </p>

  <p>
    For takedown requests, account recovery, or anything that needs urgent attention, use the same
    address with <span class="t-mono">[urgent]</span> in the subject.
  </p>

  <p class="t-meta">We answer within two business days.</p>
</main>

<AppFooter />

<style>
  .static-page {
    max-width: 720px;
    margin: 0 auto;
    padding: 80px 32px 96px;
  }
  .static-page h1 {
    font-family: var(--font-display);
    font-size: 48px;
    font-weight: 400;
    line-height: 1.1;
    margin: 16px 0 32px;
  }
  .static-page p {
    font-size: 16px;
    line-height: 1.7;
    color: var(--fg-secondary);
    margin: 0 0 20px;
  }
  .static-page a {
    color: var(--accent);
  }
  .static-page .t-mono {
    font-family: var(--font-mono);
    font-size: 0.9em;
  }
  .static-page .t-meta {
    margin-top: 48px;
    color: var(--fg-muted);
  }
  @media (max-width: 640px) {
    .static-page {
      padding: 48px 24px 64px;
    }
    .static-page h1 {
      font-size: 32px;
    }
  }
</style>
