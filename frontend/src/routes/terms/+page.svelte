<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
</script>

<svelte:head><title>Terms — Astrophoto</title></svelte:head>

<AppHeader />

<main class="static-page">
  <div class="t-eyebrow accent">● TERMS OF SERVICE</div>
  <h1>The short version.</h1>

  <p>
    By using Astrophoto you agree to upload only images you have the rights to share, to be civil in
    comments and discussion, and to respect other photographers' work.
  </p>

  <p>
    Your photos remain yours. We store them on AWS S3, deliver them via CloudFront, and keep them
    for as long as your account is active. You can delete a photo or your entire account at any time
    from <a href="/settings">settings</a>.
  </p>

  <p>The full terms are in the works. Until they ship, the principles above apply.</p>

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
