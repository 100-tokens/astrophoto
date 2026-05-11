<script lang="ts">
  import Section from '$lib/components/settings/Section.svelte';
  let { data } = $props();
  let confirming = $state(false);

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

  let otherCount = $derived(data.sessions.filter((s) => !s.is_current).length);
</script>

<Section title="Active sessions" description="Devices currently signed in to this account.">
  <ul class="sessions" role="list">
    {#each data.sessions as s (s.id)}
      <li class="session-row" class:current={s.is_current}>
        <span class="dot" class:on={s.is_current}></span>
        <div class="info">
          <strong
            >{s.os} · {s.browser}{#if s.is_current}
              <em class="muted-accent">· this device</em>{/if}</strong
          >
          <span class="meta">{s.browser} {s.browser_version} · {s.os} {s.os_version}</span>
          <span class="meta">IP {s.ip} · {relative(s.last_used_at)}</span>
        </div>
        {#if !s.is_current}
          <form method="POST" action="?/revoke">
            <input type="hidden" name="id" value={s.id} />
            <button class="btn btn-danger btn-sm" aria-label="Revoke session: {s.os} {s.browser}"
              >Revoke</button
            >
          </form>
        {/if}
      </li>
    {/each}
  </ul>

  {#if otherCount > 0}
    {#if confirming}
      <form method="POST" action="?/signOutOthers" class="confirm-form">
        <p>End {otherCount} other session(s)?</p>
        <button class="btn btn-secondary">Confirm sign-out</button>
        <button
          type="button"
          onclick={() => {
            confirming = false;
          }}>Cancel</button
        >
      </form>
    {:else}
      <button
        class="btn btn-secondary"
        onclick={() => {
          confirming = true;
        }}
      >
        Sign out of all other sessions
      </button>
    {/if}
  {/if}
</Section>

<style>
  .sessions {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .session-row {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 16px 0;
    border-bottom: 1px solid var(--border-subtle);
  }
  .session-row:last-child {
    border-bottom: none;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--border-default);
    flex-shrink: 0;
    margin-top: 6px;
  }
  .dot.on {
    background: var(--accent);
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
  .muted-accent {
    font-style: normal;
    color: var(--accent);
    font-weight: 400;
  }
  .confirm-form {
    margin-top: 16px;
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .confirm-form p {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-muted);
  }
</style>
