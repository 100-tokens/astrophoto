<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
</script>

<svelte:head><title>About — Astrophoto</title></svelte:head>

<AppHeader active="About" />

<main class="static-page">
  <div class="t-eyebrow accent">● ABOUT</div>
  <h1>A quiet archive of the night sky.</h1>

  <p>
    Astrophoto is a home for amateur astrophotographers — a place where an 18-hour integration of
    NGC 7000 looks as monumental as it actually is, and where every frame carries its full record:
    target, equipment, sky.
  </p>

  <p>
    We're a small team building tools we wanted ourselves. No ads, no rankings, no algorithm pushing
    your work down the feed. Your photos with their full technical record, kept for as long as you
    want them kept.
  </p>

  <p class="t-meta">Astrophoto · Est. MMXXVI · 52°31′N · 13°24′E · Bortle 8</p>
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
