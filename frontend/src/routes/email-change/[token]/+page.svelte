<script lang="ts">
  import type { PageProps } from './$types';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import Button from '$lib/components/Button.svelte';

  let { data }: PageProps = $props();
</script>

<svelte:head>
  <title>Email change — Astrophoto</title>
</svelte:head>

<AppHeader />

<main>
  <div class="confirm-screen">
    <div class="confirm-col">
      {#if data.status === 'expired'}
        <div class="panel panel-danger">
          <h1 class="panel-title">Link expired</h1>
          <p class="panel-body">
            Email-change confirmation links expire after 24 hours. Please request a new email change
            from your account settings.
          </p>
          <Button variant="primary" href="/settings/email">Back to settings</Button>
        </div>
      {:else if data.status === 'taken'}
        <div class="panel panel-danger">
          <h1 class="panel-title">Address already taken</h1>
          <p class="panel-body">
            That email address is already associated with another account. Please choose a different
            address in your account settings.
          </p>
          <Button variant="primary" href="/settings/email">Back to settings</Button>
        </div>
      {:else}
        <div class="panel panel-danger">
          <h1 class="panel-title">Something went wrong</h1>
          <p class="panel-body">
            We could not confirm your email change. The link may be invalid or already used.
          </p>
          <Button variant="primary" href="/settings/email">Back to settings</Button>
        </div>
      {/if}
    </div>
  </div>
</main>

<style>
  .confirm-screen {
    min-height: 100dvh;
    background: var(--bg-canvas);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: 64px;
  }

  .confirm-col {
    width: 100%;
    max-width: 480px;
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 32px;
    background: var(--bg-surface);
    border-radius: 4px;
    margin-top: 80px;
  }

  .panel-danger {
    border: 1px solid var(--danger);
  }

  .panel-title {
    font-family: var(--font-display);
    font-size: 28px;
    font-weight: 600;
    margin: 0;
    color: var(--fg-primary);
  }

  .panel-body {
    color: var(--fg-secondary);
    margin: 0;
    font-size: 15px;
    line-height: 1.6;
  }

  @media (max-width: 768px) {
    .confirm-screen {
      padding: 40px 24px;
    }
  }

  @media (max-width: 480px) {
    .confirm-screen {
      padding: 32px 16px;
    }
  }
</style>
