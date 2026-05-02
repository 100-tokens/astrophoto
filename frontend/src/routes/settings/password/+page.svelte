<script lang="ts">
  import Section from '$lib/components/settings/Section.svelte';

  let { form } = $props();
</script>

<Section title="Change password">
  <form method="POST">
    <label class="lbl" for="current_password">CURRENT PASSWORD</label>
    <input type="password" id="current_password" name="current_password" class="input" />
    <a class="link-accent" href="/reset">I don't remember it →</a>

    <label class="lbl" for="new_password">NEW PASSWORD</label>
    <input
      type="password"
      id="new_password"
      name="new_password"
      required
      minlength="12"
      class="input"
    />

    {#if form?.error === 'wrong_password'}<p class="err">Wrong current password.</p>{/if}
    {#if form?.error === 'too_short'}<p class="err">Use at least 12 characters.</p>{/if}
    {#if form?.error === 'too_common'}<p class="err">
        That password is too common. Choose something more unique.
      </p>{/if}
    {#if form?.error === 'throttled'}<p class="err">
        Too many requests. Wait a minute and try again.
      </p>{/if}
    {#if form?.error === 'invalid'}<p class="err">That value isn't valid.</p>{/if}
    {#if form?.error === 'server'}<p class="err">Something went wrong. Please try again.</p>{/if}
    {#if form?.ok}<p class="ok">Password changed. Other devices have been signed out.</p>{/if}

    <button type="submit" class="btn btn-primary">Save new password</button>
  </form>
</Section>

<style>
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
  .link-accent {
    display: block;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--accent);
    text-decoration: none;
    margin-bottom: 16px;
  }
  .link-accent:hover {
    text-decoration: underline;
  }
  .err {
    color: var(--danger);
    font-family: var(--font-mono);
    font-size: 12px;
    margin: 8px 0;
  }
  .ok {
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: 12px;
    margin: 8px 0;
  }
</style>
