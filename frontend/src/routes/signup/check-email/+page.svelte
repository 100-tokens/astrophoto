<script lang="ts">
  import { page } from '$app/state';
  import { enhance } from '$app/forms';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import Button from '$lib/components/Button.svelte';

  let email = $derived(page.url.searchParams.get('email') ?? '');
  let expired = $derived(page.url.searchParams.get('expired') === '1');
  let secondsLeft = $state(60);
  let resending = $state(false);
  let resentOk = $state(false);

  $effect(() => {
    const t = setInterval(() => {
      secondsLeft = Math.max(0, secondsLeft - 1);
    }, 1000);
    return () => clearInterval(t);
  });
</script>

<svelte:head>
  <title>Check your email — Astrophoto</title>
</svelte:head>

<AppHeader />

<div class="check-email-screen">
  <div class="check-email-col">
    <div class="t-eyebrow" style="margin-bottom: 16px;">SIGN UP</div>
    <h1 class="t-h1">Check your email</h1>
    {#if expired}
      <p class="t-body" style="color: var(--color-warning, #c47);">
        That verification link has expired or was already used. Click resend to get a new one.
      </p>
    {/if}
    <p class="t-body">
      We sent a confirmation link to <strong>{email}</strong>. Open it to finish setting up your
      account. It can take a minute to arrive — don't forget to check spam.
    </p>

    <form
      method="POST"
      action="?/resend"
      use:enhance={() =>
        async ({ result }) => {
          resending = false;
          if (result.type === 'success') resentOk = true;
          secondsLeft = 60;
        }}
      onsubmit={() => {
        resending = true;
      }}
    >
      <input type="hidden" name="email" value={email} />
      <Button type="submit" disabled={resending || secondsLeft > 0}>
        {#if resending}
          Sending…
        {:else if secondsLeft > 0}
          Resend in {secondsLeft}s
        {:else}
          Resend confirmation
        {/if}
      </Button>
      {#if resentOk}
        <p class="t-body" style="margin-top: 8px;">
          If your account exists and isn't yet verified, we sent another link.
        </p>
      {/if}
    </form>

    <p class="t-body" style="margin-top: 24px;">
      <a href="/signin">Back to sign in</a>
    </p>
  </div>
</div>

<style>
  .check-email-screen {
    display: flex;
    justify-content: center;
    padding: 64px 24px;
  }
  .check-email-col {
    max-width: 480px;
    width: 100%;
  }
</style>
