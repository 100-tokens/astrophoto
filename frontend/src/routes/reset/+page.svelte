<script lang="ts">
  import type { PageProps } from './$types';
  import Wordmark from '$lib/components/Wordmark.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';

  let { form }: PageProps = $props();
</script>

<svelte:head>
  <title>Reset password — Astrophoto</title>
</svelte:head>

<div class="reset-screen">
  <div class="reset-col">
    <div class="reset-logo">
      <Wordmark size={28} italic={true} />
    </div>

    <div class="t-eyebrow" style="margin-top: 48px; margin-bottom: 16px;">PASSWORD RESET</div>

    <h1 class="reset-headline">
      We'll send you a link <em>to find your way back.</em>
    </h1>

    <p class="reset-body">Single-use. Expires in one hour.</p>

    <form method="POST" class="reset-form">
      <div class="field">
        <label class="t-label" for="email">EMAIL</label>
        <Input name="email" id="email" type="email" required placeholder="you@domain.com" />
      </div>

      {#if form?.error === 'missing_email'}
        <p class="t-meta form-error">Please enter a valid email.</p>
      {/if}

      <div style="margin-top: 8px;">
        <Button variant="primary" size="lg" type="submit" class="full-width-btn">
          Send reset link
        </Button>
      </div>

      <p class="t-meta back-link">
        <a href="/signin" style="color: var(--accent);">← Back to sign in</a>
      </p>
    </form>
  </div>
</div>

<style>
  .reset-screen {
    min-height: 100dvh;
    background: var(--bg-canvas);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: 64px;
  }

  .reset-col {
    width: 100%;
    max-width: 480px;
    display: flex;
    flex-direction: column;
  }

  .reset-logo {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .reset-headline {
    font-family: var(--font-display);
    font-size: 44px;
    font-weight: 600;
    margin: 0;
    line-height: 1.05;
    color: var(--fg-primary);
  }

  .reset-headline em {
    font-style: italic;
  }

  .reset-body {
    margin-top: 16px;
    color: var(--fg-secondary);
    font-size: 16px;
    line-height: 1.6;
  }

  .reset-form {
    display: flex;
    flex-direction: column;
    gap: 16px;
    margin-top: 40px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .form-error {
    color: var(--danger);
    margin: 0;
  }

  .back-link {
    text-align: center;
    margin-top: 4px;
  }

  /* Make Button full-width within form context */
  :global(.full-width-btn) {
    width: 100%;
    justify-content: center;
  }

  @media (max-width: 768px) {
    .reset-screen {
      padding: 40px 24px;
    }

    .reset-headline {
      font-size: 36px;
    }
  }

  @media (max-width: 480px) {
    .reset-screen {
      padding: 32px 16px;
    }

    .reset-headline {
      font-size: 32px;
    }
  }
</style>
