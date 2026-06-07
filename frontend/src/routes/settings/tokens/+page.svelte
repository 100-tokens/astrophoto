<script lang="ts">
  import { enhance } from '$app/forms';
  import Section from '$lib/components/settings/Section.svelte';

  let { data, form } = $props();
  let copied = $state(false);

  function relative(iso: string): string {
    const dt = (Date.now() - new Date(iso).getTime()) / 1000;
    if (dt < 60) return 'just now';
    if (dt < 3600) {
      const m = Math.floor(dt / 60);
      return `${m} minute${m === 1 ? '' : 's'} ago`;
    }
    if (dt < 86400) {
      const h = Math.floor(dt / 3600);
      return `${h} hour${h === 1 ? '' : 's'} ago`;
    }
    const d = Math.floor(dt / 86400);
    return `${d} day${d === 1 ? '' : 's'} ago`;
  }

  function created(iso: string): string {
    return new Date(iso).toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric'
    });
  }

  async function copySecret(secret: string) {
    try {
      await navigator.clipboard.writeText(secret);
      copied = true;
    } catch {
      copied = false;
    }
  }
</script>

<Section
  title="PixInsight & API tokens"
  description="Personal access tokens let external tools publish to Astrophoto on your behalf."
>
  <p class="explainer">
    Generate a token here, then paste it into the PixInsight
    <strong>“Publish to Astrophoto”</strong> dialog. Each token acts as your account — keep it secret,
    and revoke it if a machine is lost or compromised.
  </p>

  {#if form && 'created' in form}
    {@const newToken = form.created}
    <div class="reveal" role="status">
      <span class="eyebrow">● NEW TOKEN · COPY IT NOW</span>
      <p class="reveal-name">{newToken.name}</p>
      <code class="secret">{newToken.secret}</code>
      <div class="reveal-actions">
        <button
          type="button"
          class="btn btn-secondary btn-sm"
          onclick={() => copySecret(newToken.secret)}
        >
          {copied ? 'Copied' : 'Copy'}
        </button>
      </div>
      <p class="warn">
        This is the only time the full token is shown. Store it now — it cannot be retrieved again.
      </p>
    </div>
  {/if}

  <form method="POST" action="?/create" class="create-form" use:enhance>
    <label class="lbl" for="name">TOKEN NAME</label>
    <input
      type="text"
      id="name"
      name="name"
      class="input"
      maxlength="80"
      required
      placeholder="PixInsight on iMac"
    />
    {#if form && 'error' in form && form.error === 'name_required'}<p class="err">
        Give the token a name.
      </p>{/if}
    {#if form && 'error' in form && form.error === 'invalid'}<p class="err">
        That name isn't valid.
      </p>{/if}
    {#if form && 'error' in form && form.error === 'server'}<p class="err">
        Something went wrong. Please try again.
      </p>{/if}
    <button type="submit" class="btn btn-primary">Generate token</button>
  </form>

  <ul class="tokens" role="list">
    {#each data.tokens as t (t.id)}
      <li class="token-row" class:revoked={!!t.revoked_at}>
        <div class="info">
          <strong>{t.name}</strong>
          <span class="meta">{t.prefix}… · created {created(t.created_at)}</span>
          {#if t.last_used_at}
            <span class="meta">last used {relative(t.last_used_at)}</span>
          {:else}
            <span class="meta">never used</span>
          {/if}
        </div>
        {#if t.revoked_at}
          <span class="revoked-tag">(revoked)</span>
        {:else}
          <form method="POST" action="?/revoke" use:enhance>
            <input type="hidden" name="id" value={t.id} />
            <button class="btn btn-danger btn-sm" aria-label="Revoke token: {t.name}">Revoke</button
            >
          </form>
        {/if}
      </li>
    {/each}
    {#if data.tokens.length === 0}
      <li class="empty">No tokens yet.</li>
    {/if}
  </ul>
</Section>

<style>
  .explainer {
    font-size: 13px;
    color: var(--fg-muted);
    max-width: 560px;
    margin: 0 0 24px;
  }
  .eyebrow {
    font-family: var(--font-mono);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    color: var(--fg-muted);
  }
  .reveal {
    border: 1px solid var(--accent);
    background: var(--bg-accent-tint);
    border-radius: 4px;
    padding: 16px;
    margin-bottom: 24px;
  }
  .reveal-name {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-primary);
    margin: 8px 0 4px;
  }
  .secret {
    display: block;
    font-family: var(--font-mono);
    font-size: 13px;
    color: var(--fg-primary);
    background: var(--bg-base);
    border: 1px solid var(--border-default);
    border-radius: 4px;
    padding: 10px 12px;
    word-break: break-all;
    user-select: all;
  }
  .reveal-actions {
    margin-top: 12px;
  }
  .warn {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--danger);
    margin: 12px 0 0;
  }
  .create-form {
    margin-bottom: 32px;
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
    margin-bottom: 16px;
  }
  .err {
    color: var(--danger);
    font-family: var(--font-mono);
    font-size: 12px;
    margin: 8px 0;
  }
  .tokens {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .token-row {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 16px 0;
    border-bottom: 1px solid var(--border-subtle);
  }
  .token-row:last-child {
    border-bottom: none;
  }
  .token-row.revoked .info {
    color: var(--fg-muted);
  }
  .info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
  }
  .meta {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
  }
  .revoked-tag {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-muted);
  }
  .empty {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-muted);
    padding: 16px 0;
  }
</style>
