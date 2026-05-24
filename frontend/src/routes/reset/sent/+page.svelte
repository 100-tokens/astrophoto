<script lang="ts">
  import { page } from '$app/state';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import Button from '$lib/components/Button.svelte';
  import { api } from '$lib/api/client';

  let email = $derived(page.url.searchParams.get('email') ?? '');
  let secondsLeft = $state(60);
  let resending = $state(false);
  let resentOk = $state(false);

  $effect(() => {
    const t = setInterval(() => {
      secondsLeft = Math.max(0, secondsLeft - 1);
    }, 1000);
    return () => clearInterval(t);
  });

  async function resend() {
    resending = true;
    try {
      await api.passwordResetRequest(email);
    } catch {
      // Best-effort; we don't reveal whether the address exists.
    }
    resending = false;
    resentOk = true;
    secondsLeft = 60;
  }
</script>

<svelte:head>
  <title>Check your email — Astrophoto</title>
</svelte:head>

<AppHeader />

<main>
  <div class="reset-screen">
    <div class="reset-col">
      <div class="t-eyebrow" style="margin-bottom: 16px;">PASSWORD RESET</div>

      <h1 class="reset-headline">
        A link is on its way <em>to {email || 'your inbox'}.</em>
      </h1>

      <p class="reset-body">Open the email and click the link to set a new password.</p>

      <pre class="email-preview">
EMAIL PREVIEW · PLAIN TEXT

From:    Astrophoto &lt;noreply@astrophoto.pics&gt;
To:      {email}
Subject: Reset your Astrophoto password

Open this link to choose a new password:

  https://astrophoto.pics/reset/&lt;your-token&gt;

The link is single-use and expires in one hour.</pre>

      <div class="actions">
        {#if secondsLeft > 0}
          <Button variant="ghost" disabled>
            Resend in 0:{String(secondsLeft).padStart(2, '0')}
          </Button>
        {:else}
          <Button variant="ghost" onclick={resend} disabled={resending}>
            {resending ? 'Sending…' : resentOk ? 'Sent again ✓' : 'Resend link'}
          </Button>
        {/if}
        <Button variant="secondary" href="/reset">Use a different email</Button>
      </div>
    </div>
  </div>
</main>

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
    max-width: 560px;
    display: flex;
    flex-direction: column;
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

  .email-preview {
    margin-top: 32px;
    padding: 24px;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 13px;
    line-height: 1.7;
    color: var(--fg-secondary);
    white-space: pre-wrap;
    overflow-x: auto;
  }

  .actions {
    display: flex;
    gap: 12px;
    margin-top: 32px;
    flex-wrap: wrap;
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

    .actions {
      flex-direction: column;
    }
  }
</style>
