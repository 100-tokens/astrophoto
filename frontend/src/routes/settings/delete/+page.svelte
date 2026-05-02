<script lang="ts">
  import Section from '$lib/components/settings/Section.svelte';

  let { data, form } = $props();
  let phrase = $state('');

  const exportHref = `${import.meta.env.VITE_API_BASE_URL ?? ''}/api/me/export.json`;
</script>

{#if data.pending_deletion_at}
  <div class="panel danger">
    <span class="eyebrow">● DELETION SCHEDULED</span>
    <p>
      Your account will be permanently erased on {new Date(
        data.pending_deletion_at
      ).toLocaleString()}.
    </p>
    <form method="POST" action="?/cancel">
      <button class="btn btn-primary">Cancel deletion · keep my account</button>
    </form>
    <a class="btn btn-secondary" href={exportHref} download>Download my archive (JSON)</a>
  </div>
{:else}
  <Section
    title="Delete account"
    tone="danger"
    description="Closing your account erases your photos, comments, and identity. There is a 7-day grace period."
  >
    <form method="POST" action="?/request">
      <label class="lbl" for="current_password">CURRENT PASSWORD</label>
      <input type="password" id="current_password" name="current_password" class="input" />
      <label class="lbl" for="confirmation_phrase">TYPE "DELETE MY ACCOUNT" TO CONFIRM</label>
      <input
        type="text"
        id="confirmation_phrase"
        name="confirmation_phrase"
        bind:value={phrase}
        class="input"
      />
      {#if form?.error === 'phrase'}<p class="err">The phrase doesn't match.</p>{/if}
      {#if form?.error === 'wrong_password'}<p class="err">Wrong password.</p>{/if}
      {#if form?.error === 'throttled'}<p class="err">
          Too many requests. Wait a minute and try again.
        </p>{/if}
      {#if form?.error === 'invalid'}<p class="err">That value isn't valid.</p>{/if}
      {#if form?.error === 'server'}<p class="err">Something went wrong. Please try again.</p>{/if}
      <button type="submit" class="btn btn-danger" disabled={phrase !== 'DELETE MY ACCOUNT'}>
        Begin 7-day deletion
      </button>
    </form>
  </Section>
{/if}

<style>
  .panel {
    padding: 24px;
    border: 1px solid var(--border-default);
    border-radius: var(--r-md, 4px);
  }
  .panel.danger {
    border-color: var(--danger);
    background: var(--bg-danger-tint, transparent);
  }
  .eyebrow {
    display: block;
    font-family: var(--font-mono);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    color: var(--danger);
    margin-bottom: 12px;
  }
  .panel p {
    font-size: 14px;
    color: var(--fg-primary);
    margin: 0 0 16px;
  }
  .panel form {
    margin-bottom: 12px;
  }
  .lbl {
    display: block;
    font-family: var(--font-mono);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    color: var(--fg-muted);
    margin: 16px 0 4px;
  }
  .input {
    width: 100%;
    box-sizing: border-box;
    margin-bottom: 8px;
  }
  .err {
    color: var(--danger);
    font-family: var(--font-mono);
    font-size: 12px;
    margin: 8px 0;
  }
</style>
