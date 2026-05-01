<script lang="ts">
  import type { PageProps } from './$types';
  import MarkReticle from '$lib/components/MarkReticle.svelte';
  import Wordmark from '$lib/components/Wordmark.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';

  let { form }: PageProps = $props();
</script>

<svelte:head>
  <title>Open an account — Astrophoto</title>
</svelte:head>

<div class="signup-screen">
  <div class="signup-col">
    <!-- Logo -->
    <div class="signup-logo">
      <MarkReticle size={28} color="var(--accent)" />
      <Wordmark size={22} italic={true} />
    </div>

    <!-- Eyebrow -->
    <div class="t-eyebrow" style="color: var(--accent); margin-top: 48px; margin-bottom: 16px;">
      ● OPEN AN ACCOUNT
    </div>

    <!-- Headline -->
    <h1 class="signup-headline">
      A serious home for<br /><em>the work you make</em>.
    </h1>

    <!-- Reassurance -->
    <p class="signup-body">
      Free to use. We treat every frame as a technical record and as a photograph. You keep all your
      data; you can delete your account at any time.
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
        <Input name="display_name" id="display_name" required placeholder="Marie Dubois" />
      </div>

      <div class="field">
        <label class="t-label" for="email">EMAIL</label>
        <Input name="email" id="email" type="email" required placeholder="you@domain.com" />
      </div>

      <div class="field">
        <label class="t-label" for="password">PASSWORD</label>
        <Input
          name="password"
          id="password"
          type="password"
          required
          placeholder="At least 12 characters"
        />
        <span class="t-meta password-hint">Use at least 12 characters.</span>
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
        By creating an account you agree to the
        <a href="/terms" style="color: var(--accent);">Terms</a>
        and
        <a href="/privacy" style="color: var(--accent);">Privacy Policy</a>. We don't track you
        across the web.
      </p>
    </form>

    <!-- Already have an account -->
    <div class="signin-link">
      <span class="t-meta" style="color: var(--fg-muted);">
        Already have an account?
        <a href="/signin" style="color: var(--accent);">Sign in →</a>
      </span>
    </div>
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

  .password-hint {
    margin-top: 2px;
    font-size: 12px;
    color: var(--fg-muted);
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

  .signin-link {
    margin-top: 32px;
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
