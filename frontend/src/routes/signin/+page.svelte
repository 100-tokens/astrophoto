<script lang="ts">
  import type { PageProps } from './$types';
  import Photo from '$lib/components/Photo.svelte';
  import MarkReticle from '$lib/components/MarkReticle.svelte';
  import Wordmark from '$lib/components/Wordmark.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';

  let { form }: PageProps = $props();
</script>

<svelte:head>
  <title>Sign in — Astrophoto</title>
</svelte:head>

<div class="signin-screen">
  <!-- Left column: photo + quote -->
  <div class="photo-col">
    <Photo
      target="ρ Ophiuchi Cloud"
      style="position: absolute; inset: 0; width: 100%; height: 100%;"
    />
    <div class="photo-overlay"></div>

    <!-- Logo top-left -->
    <div class="photo-logo">
      <MarkReticle size={28} color="var(--fg-primary)" />
      <Wordmark size={22} italic={true} />
    </div>

    <!-- Quote bottom-left -->
    <div class="photo-quote">
      <div class="t-eyebrow" style="color: var(--accent); margin-bottom: 16px;">
        ● ρ OPHIUCHI · 5h45m · A. DIMOV
      </div>
      <p class="quote-text">"The faintest tendrils of dust only show themselves to the patient."</p>
    </div>

    <!-- Coordinates bottom-right -->
    <div class="photo-coords">16ʰ 25ᵐ / −23° 27′</div>
  </div>

  <!-- Right column: form -->
  <div class="form-col">
    <div class="form-inner">
      <div class="t-eyebrow" style="margin-bottom: 16px;">SIGN IN</div>

      <h1 class="signin-headline">
        Welcome back<br />to <em>your archive</em>.
      </h1>

      <p class="signin-subline">
        New here? <a href="/signup" style="color: var(--accent);">Open an account →</a>
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
        <span class="t-meta">OR</span>
        <div class="divider-line"></div>
      </div>

      <!-- Email/password form -->
      <form method="POST" class="auth-form">
        <div class="field">
          <label class="t-label" for="email">EMAIL</label>
          <Input name="email" id="email" type="email" required placeholder="you@domain.com" />
        </div>

        <div class="field">
          <div class="field-label-row">
            <label class="t-label" for="password">PASSWORD</label>
            <a href="/reset" class="t-meta" style="color: var(--accent);">Forgot?</a>
          </div>
          <Input
            name="password"
            id="password"
            type="password"
            required
            placeholder="••••••••••••"
          />
        </div>

        {#if form?.message}
          <p class="t-meta form-error">{form.message}</p>
        {/if}

        <div style="margin-top: 8px;">
          <Button variant="primary" size="lg" type="submit" class="full-width-btn">Sign in</Button>
        </div>
      </form>
    </div>
  </div>
</div>

<style>
  .signin-screen {
    display: grid;
    grid-template-columns: 1fr 1fr;
    min-height: 100dvh;
    background: var(--bg-canvas);
  }

  /* ── Left column ─────────────────────────────────────────── */
  .photo-col {
    position: relative;
    overflow: hidden;
    background: #000;
    min-height: 100dvh;
  }

  .photo-overlay {
    position: absolute;
    inset: 0;
    background: linear-gradient(to right, rgba(0, 0, 0, 0.6) 0%, transparent 50%);
    z-index: 1;
  }

  .photo-logo {
    position: absolute;
    left: 64px;
    top: 64px;
    z-index: 2;
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .photo-quote {
    position: absolute;
    left: 64px;
    bottom: 64px;
    z-index: 2;
    max-width: 480px;
  }

  .quote-text {
    font-family: var(--font-display);
    font-size: 32px;
    font-style: italic;
    line-height: 1.15;
    margin: 0;
    color: var(--fg-primary);
  }

  .photo-coords {
    position: absolute;
    right: 24px;
    bottom: 24px;
    z-index: 2;
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--fg-muted);
    letter-spacing: 0.12em;
  }

  /* ── Right column ────────────────────────────────────────── */
  .form-col {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 64px;
  }

  .form-inner {
    width: 100%;
    max-width: 380px;
  }

  .signin-headline {
    font-family: var(--font-display);
    font-size: 44px;
    font-weight: 600;
    margin: 0;
    line-height: 1.05;
    color: var(--fg-primary);
  }

  .signin-headline em {
    font-style: italic;
  }

  .signin-subline {
    margin-top: 16px;
    color: var(--fg-secondary);
    font-size: 14px;
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

  .auth-form {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .field-label-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .form-error {
    color: var(--danger);
    margin: 0;
  }

  /* Make Button full-width within form context */
  :global(.full-width-btn) {
    width: 100%;
    justify-content: center;
  }

  /* ── Mobile ──────────────────────────────────────────────── */
  @media (max-width: 768px) {
    .signin-screen {
      grid-template-columns: 1fr;
    }

    .photo-col {
      min-height: 220px;
      max-height: 280px;
    }

    .photo-logo {
      left: 24px;
      top: 24px;
    }

    .photo-quote {
      display: none;
    }

    .photo-coords {
      display: none;
    }

    .form-col {
      padding: 40px 24px;
    }
  }

  @media (max-width: 480px) {
    .photo-col {
      min-height: 160px;
    }

    .form-col {
      padding: 32px 16px;
    }

    .signin-headline {
      font-size: 36px;
    }
  }
</style>
