<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
</script>

<svelte:head><title>Privacy — Astrophoto</title></svelte:head>

<AppHeader />

<main class="static-page">
  <div class="t-eyebrow accent">● PRIVACY</div>
  <h1>What we keep, and why.</h1>

  <p>
    We keep the minimum required to run the site: your email (for sign-in and notifications), your
    handle and display name (visible on your profile), and the photos and metadata you upload.
  </p>

  <p>
    No third-party trackers. No advertising cookies. No analytics that follow you elsewhere. Server
    logs are kept for 30 days for debugging and abuse prevention.
  </p>

  <p>
    You can request a full export or permanent deletion of your account from
    <a href="/settings">settings</a>. Account deletion has a 30-day grace period during which you
    can reverse it; after that, it's gone for good.
  </p>

  <p class="t-meta">Last updated: 2026-05-10</p>
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
