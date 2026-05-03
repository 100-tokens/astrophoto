<script lang="ts">
  import type { PageProps } from './$types';
  import Wordmark from '$lib/components/Wordmark.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';
  import HandlePicker from '$lib/components/HandlePicker.svelte';

  let { form }: PageProps = $props();

  // Preserve handle across server-side error round-trips.
  // $derived keeps this reactive when `form` changes after a server action.
  let handle = $state('');
  $effect(() => {
    if (form?.handle !== undefined) handle = form.handle;
  });
</script>

<svelte:head>
  <title>Open an account — Astrophoto</title>
</svelte:head>

<div class="signup-screen">
  <div class="signup-col">
    <!-- Logo: wordmark only (per design prototype). -->
    <div class="signup-logo">
      <Wordmark size={28} italic={true} />
    </div>

    <!-- Eyebrow: no leading dot (per design prototype) -->
    <div class="t-eyebrow" style="margin-top: 48px; margin-bottom: 16px;">OPEN AN ACCOUNT</div>

    <!-- Headline -->
    <h1 class="signup-headline">
      A serious home for<br /><em>the work you make</em>.
    </h1>

    <!-- Reassurance (verbatim from design prototype) -->
    <p class="signup-body">
      Free, no ads, no rankings. Your photos with their full technical record, kept for as long as
      you want them kept.
    </p>

    <!-- Google OAuth button -->
    <div style="margin-top: 40px;">
      <Button variant="secondary" size="lg" class="full-width-btn">
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          aria-hidden="true"
        >
          <circle cx="12" cy="12" r="10" />
          <path d="M17 12h-5v4h3a5 5 0 1 1 0-8H12" stroke-linecap="round" />
        </svg>
        Continue with Google
      </Button>
    </div>

    <!-- Divider -->
    <div class="divider">
      <div class="divider-line"></div>
      <span class="t-meta">OR WITH EMAIL</span>
      <div class="divider-line"></div>
    </div>

    <!-- Sign-up form -->
    <form method="POST" class="signup-form">
      <div class="field">
        <label class="t-label" for="display_name">DISPLAY NAME</label>
        <Input
          name="display_name"
          id="display_name"
          required
          placeholder="How others will see you"
        />
      </div>

      <div class="field">
        <HandlePicker bind:value={handle} />
        {#if form?.handleError}
          <p class="t-meta form-error">{form.handleError}</p>
        {/if}
      </div>

      <div class="field">
        <label class="t-label" for="email">EMAIL</label>
        <Input name="email" id="email" type="email" required placeholder="you@somewhere.com" />
      </div>

      <div class="field">
        <label class="t-label" for="password">PASSWORD</label>
        <Input
          name="password"
          id="password"
          type="password"
          required
          placeholder="At least 10 characters"
        />
      </div>

      {#if form?.message}
        <p class="t-meta form-error">{form.message}</p>
      {/if}

      <div style="margin-top: 8px;">
        <Button variant="primary" size="lg" type="submit" class="full-width-btn">
          Create my account
        </Button>
      </div>

      <p class="t-meta terms-copy">
        By continuing you agree to our
        <a href="/terms" style="color: var(--accent);">terms</a>
        and
        <a href="/privacy" style="color: var(--accent);">privacy policy</a>. We don't ask for, and
        never sell, your data.
      </p>
    </form>
  </div>
</div>

<style>
  .signup-screen {
    min-height: 100dvh;
    background: var(--bg-canvas);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: 64px;
  }

  .signup-col {
    width: 100%;
    max-width: 480px;
    display: flex;
    flex-direction: column;
  }

  .signup-logo {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .signup-headline {
    font-family: var(--font-display);
    font-size: 44px;
    font-weight: 600;
    margin: 0;
    line-height: 1.05;
    color: var(--fg-primary);
  }

  .signup-headline em {
    font-style: italic;
  }

  .signup-body {
    margin-top: 16px;
    color: var(--fg-secondary);
    font-size: 16px;
    line-height: 1.6;
    max-width: 480px;
  }

  .divider {
    display: flex;
    align-items: center;
    gap: 16px;
    margin: 24px 0;
    color: var(--fg-faint);
  }

  .divider-line {
    flex: 1;
    height: 1px;
    background: var(--border-subtle);
  }

  .signup-form {
    display: flex;
    flex-direction: column;
    gap: 16px;
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

  .terms-copy {
    margin-top: 4px;
    line-height: 1.6;
    text-align: center;
  }

  /* Make Button full-width within form context */
  :global(.full-width-btn) {
    width: 100%;
    justify-content: center;
  }

  /* ── Mobile ──────────────────────────────────────────────── */
  @media (max-width: 768px) {
    .signup-screen {
      padding: 40px 24px;
      align-items: flex-start;
    }

    .signup-headline {
      font-size: 36px;
    }
  }

  @media (max-width: 480px) {
    .signup-screen {
      padding: 32px 16px;
    }

    .signup-headline {
      font-size: 32px;
    }
  }
</style>
