<script lang="ts">
  import { enhance } from '$app/forms';
  import type { PageProps } from './$types';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';

  // data.token is read server-side by the action; unused in the template.
  let { form }: PageProps = $props();
  let pwd = $state('');

  function strength(p: string): number {
    if (p.length < 8) return 1;
    if (p.length < 12) return 2;
    if (p.length < 16) return 3;
    return 4;
  }
</script>

<svelte:head>
  <title>Choose a new password — Astrophoto</title>
</svelte:head>

<AppHeader />

<main>
  {#if form?.error === 'expired_or_used'}
    <div class="reset-screen">
      <div class="reset-col">
        <div class="panel-danger">
          <h2 class="panel-title">Link expired or already used</h2>
          <p class="panel-body">Password-reset links are single-use and expire after one hour.</p>
          <Button variant="primary" href="/reset">Request a new link</Button>
        </div>
      </div>
    </div>
  {:else}
    <div class="reset-screen">
      <div class="reset-col">
        <div class="t-eyebrow" style="margin-bottom: 16px;">PASSWORD RESET</div>

        <h1 class="reset-headline">
          Choose a <em>new password</em>.
        </h1>

        <form method="POST" use:enhance class="reset-form">
          <div class="field">
            <label class="t-label" for="new_password">NEW PASSWORD</label>
            <Input
              name="new_password"
              id="new_password"
              type="password"
              required
              placeholder="At least 12 characters"
              bind:value={pwd}
            />
          </div>

          {#if pwd.length > 0}
            <div class="strength-bar" aria-label="Password strength">
              {#each [1, 2, 3, 4] as bucket}
                <span class="strength-seg" class:on={strength(pwd) >= bucket}></span>
              {/each}
            </div>
          {/if}

          {#if pwd.length > 0 && pwd.length < 12}
            <p class="t-meta warn">Use at least 12 characters.</p>
          {/if}

          {#if form?.error === 'too_short'}
            <p class="t-meta form-error">Password must be at least 12 characters.</p>
          {/if}

          {#if form?.error === 'weak'}
            <p class="t-meta form-error">
              {#if form.detail === 'password_too_common'}
                That password is too common. Try a longer or more unique one.
              {:else}
                Password is too weak. Try a longer one with more variety.
              {/if}
            </p>
          {/if}

          {#if form?.error === 'server'}
            <p class="t-meta form-error">Something went wrong. Please try again.</p>
          {/if}

          <div style="margin-top: 8px;">
            <Button variant="primary" size="lg" type="submit" class="full-width-btn">
              Set new password &amp; sign in
            </Button>
          </div>
        </form>
      </div>
    </div>
  {/if}
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
    max-width: 480px;
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

  .strength-bar {
    display: flex;
    gap: 4px;
  }

  .strength-seg {
    flex: 1;
    height: 3px;
    background: var(--border-subtle);
    border-radius: 2px;
    transition: background 0.2s;
  }

  .strength-seg.on {
    background: var(--accent);
  }

  .warn {
    color: var(--fg-secondary);
    margin: 0;
  }

  .form-error {
    color: var(--danger);
    margin: 0;
  }

  .panel-danger {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 32px;
    background: var(--bg-surface);
    border: 1px solid var(--danger);
    border-radius: 4px;
    margin-top: 80px;
  }

  .panel-title {
    font-family: var(--font-display);
    font-size: 24px;
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
