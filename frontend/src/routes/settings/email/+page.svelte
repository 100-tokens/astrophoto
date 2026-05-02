<script lang="ts">
  import Section from '$lib/components/settings/Section.svelte';
  import Row from '$lib/components/settings/Row.svelte';
  import Modal from '$lib/components/Modal.svelte';

  let { data, form } = $props();
  let showModal = $state(false);
</script>

<Section title="Sign-in identity" description="The email used to sign in and recover your account.">
  <Row label="EMAIL">
    <span class="value">{data.user?.email}</span>
    <button
      class="btn btn-secondary btn-sm"
      onclick={() => {
        showModal = true;
      }}>Change…</button
    >
  </Row>
</Section>

<Modal bind:open={showModal} title="Change email">
  <span class="eyebrow">● CHANGE EMAIL · VERIFICATION REQUIRED</span>
  <h2>Change <em>your sign-in email</em></h2>
  <form method="POST" action="?/requestChange">
    <label class="lbl" for="new_email">NEW EMAIL</label>
    <input type="email" id="new_email" name="new_email" required class="input" />
    <label class="lbl" for="current_password">CURRENT PASSWORD</label>
    <input type="password" id="current_password" name="current_password" required class="input" />
    {#if form?.error === 'wrong_password'}<p class="err">Wrong password.</p>{/if}
    {#if form?.ok}<p class="ok">Check your new inbox for a confirmation link.</p>{/if}
    <button type="submit" class="btn btn-primary">Send confirmation link</button>
  </form>
</Modal>

<style>
  .value {
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--fg-primary);
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
  h2 {
    font-family: var(--font-display);
    font-size: 24px;
    font-weight: 600;
    font-style: italic;
    margin: 8px 0 16px;
  }
  .eyebrow {
    font-family: var(--font-mono);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    color: var(--fg-muted);
  }
</style>
