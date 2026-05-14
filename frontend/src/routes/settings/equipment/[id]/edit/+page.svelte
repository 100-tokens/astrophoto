<script lang="ts">
  import SetupForm from '$lib/components/SetupForm.svelte';
  import type { PageProps } from './$types';

  let { data }: PageProps = $props();
</script>

<header class="page-header">
  <nav class="crumbs t-meta">
    <a href="/settings">Settings</a>
    <span>·</span>
    <a href="/settings/equipment">Equipment</a>
    <span>·</span>
    <span>Edit setup</span>
  </nav>
  <div class="t-eyebrow">SETTINGS · EQUIPMENT</div>
  <h1 class="page-title">Edit <em>{data.setup.name}</em></h1>
  <p class="page-intro">
    Update the gear in this setup — changes take effect on frames you apply the setup to next.
  </p>
</header>

<div class="page-body">
  <SetupForm initial={data.setup} prefill={data.prefill} setupId={data.setup.id} />
</div>

<style>
  .page-header {
    padding: 40px 64px 24px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .crumbs {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 16px;
  }

  .crumbs a {
    color: var(--fg-muted);
    text-decoration: none;
  }

  .crumbs a:hover {
    color: var(--fg-secondary);
  }

  .crumbs span {
    color: var(--fg-faint);
  }

  .t-eyebrow {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.1em;
    color: var(--fg-muted);
    text-transform: uppercase;
    margin-bottom: 8px;
  }

  .page-title {
    font-family: var(--font-display);
    font-size: 48px;
    font-weight: 400;
    margin: 0 0 0;
    line-height: 1;
    color: var(--fg-primary);
  }

  .page-title em {
    font-style: italic;
  }

  .page-intro {
    color: var(--fg-secondary);
    max-width: 720px;
    font-size: 14px;
    margin-top: 16px;
    margin-bottom: 0;
    line-height: 1.6;
  }

  .page-body {
    padding: 48px 64px;
  }
</style>
