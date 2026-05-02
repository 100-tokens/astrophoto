<script lang="ts">
  import '../app.css';

  let { data, children } = $props();
  let countdown = $state('');

  function refresh() {
    if (!data.user?.pending_deletion_at) return;
    const left = new Date(data.user.pending_deletion_at).getTime() - Date.now();
    if (left <= 0) {
      countdown = 'imminent';
      return;
    }
    const days = Math.floor(left / 86_400_000);
    const hours = Math.floor((left % 86_400_000) / 3_600_000);
    countdown = `${days} days, ${hours} hours`;
  }

  $effect(() => {
    refresh();
    const t = setInterval(refresh, 60_000);
    return () => clearInterval(t);
  });
</script>

{#if data.user?.pending_deletion_at}
  <div class="grace-banner">
    <span class="eyebrow">● ACCOUNT MARKED FOR DELETION</span>
    Permanent removal in <strong>{countdown}</strong>
    {#if data.frame_count !== null}
      · {data.frame_count} frames will be erased{/if}
    <form method="POST" action="/settings/delete?/cancel" class="cancel-form">
      <button class="link-accent">Cancel deletion</button>
    </form>
  </div>
{/if}

{@render children()}

<style>
  .grace-banner {
    background: var(--bg-danger-tint);
    border-bottom: 1px solid var(--danger);
    color: var(--fg-primary);
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 12px 64px;
    display: flex;
    gap: 24px;
    align-items: center;
  }
  .grace-banner .cancel-form {
    margin-left: auto;
  }
  .link-accent {
    color: var(--accent);
    background: none;
    border: 0;
    text-decoration: underline;
    cursor: pointer;
  }
</style>
