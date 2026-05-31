<script lang="ts">
  import { untrack } from 'svelte';
  import { invalidateAll } from '$app/navigation';
  import { updateSettings } from '$lib/api/adminClient';
  import type { AppSettings } from '$lib/api/AppSettings';

  let { data } = $props();

  // Editable draft, seeded once from the loaded settings.
  let draft = $state<AppSettings>(untrack(() => ({ ...data.settings })));
  let busy = $state(false);
  let saved = $state(false);
  let errorMsg = $state<string | null>(null);

  async function save() {
    busy = true;
    saved = false;
    errorMsg = null;
    try {
      const next = await updateSettings(fetch, {
        signups_enabled: draft.signups_enabled,
        free_upload_max_mb: Number(draft.free_upload_max_mb),
        subscriber_upload_max_mb: Number(draft.subscriber_upload_max_mb)
      });
      draft = { ...next };
      saved = true;
      await invalidateAll();
    } catch (e) {
      errorMsg = (e as Error).message;
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head><title>Settings · Admin · Astrophoto</title></svelte:head>

<header class="head"><h1>App settings</h1></header>

<form
  onsubmit={(e) => {
    e.preventDefault();
    void save();
  }}
>
  <fieldset class="section">
    <legend>Registration</legend>
    <label class="toggle">
      <input type="checkbox" bind:checked={draft.signups_enabled} />
      <span>Allow new sign-ups</span>
    </label>
    <p class="hint">When off, the sign-up form is rejected (existing users can still sign in).</p>
  </fieldset>

  <fieldset class="section">
    <legend>Upload limits</legend>
    <label class="field">
      <span>Free tier — max upload (MB)</span>
      <input type="number" min="1" max="100000" bind:value={draft.free_upload_max_mb} />
    </label>
    <label class="field">
      <span>Subscriber tier — max upload (MB)</span>
      <input type="number" min="1" max="100000" bind:value={draft.subscriber_upload_max_mb} />
    </label>
    <p class="hint">Per-file ceiling enforced when a photo upload is initialised.</p>
  </fieldset>

  {#if errorMsg}<p class="err">{errorMsg}</p>{/if}

  <div class="actions">
    <button type="submit" disabled={busy}>{busy ? 'Saving…' : 'Save settings'}</button>
    {#if saved && !busy}<span class="ok">Saved ✓</span>{/if}
  </div>
</form>

<style>
  h1 {
    font-family: var(--font-display, serif);
    font-weight: 400;
    margin: 0 0 16px;
  }
  .section {
    border: 1px solid var(--border-subtle);
    padding: 16px;
    margin: 0 0 16px;
  }
  legend {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-muted);
    padding: 0 6px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 12px;
    max-width: 360px;
  }
  .field span {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
  }
  .field input {
    background: var(--bg-canvas);
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 8px 10px;
    font-size: 14px;
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 14px;
    color: var(--fg-primary);
    cursor: pointer;
  }
  .hint {
    color: var(--fg-muted);
    font-family: var(--font-mono);
    font-size: 11px;
    margin: 8px 0 0;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  button {
    background: var(--accent);
    color: var(--accent-ink);
    border: 0;
    padding: 10px 18px;
    font-family: var(--font-mono);
    font-size: 12px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .ok {
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .err {
    color: var(--danger, #c33);
    font-family: var(--font-mono);
    font-size: 12px;
  }
</style>
