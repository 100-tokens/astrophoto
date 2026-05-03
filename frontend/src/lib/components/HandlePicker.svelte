<script lang="ts">
  interface Props {
    name?: string;
    value?: string;
    api?: string;
  }

  let {
    name = 'handle',
    value = $bindable(''),
    api = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '',
  }: Props = $props();

  type Status = 'empty' | 'checking' | 'available' | 'taken' | 'invalid' | 'reserved';
  let status: Status = $state('empty');

  // Incremented on each effect run; lets us discard stale fetch responses.
  let reqId = 0;

  $effect(() => {
    if (!value) {
      status = 'empty';
      return;
    }
    status = 'checking';
    const myId = ++reqId;
    const t = setTimeout(async () => {
      try {
        const r = await fetch(`${api}/api/auth/handle-check?handle=${encodeURIComponent(value)}`);
        const j = (await r.json()) as { status: Status };
        if (myId === reqId) status = j.status;
      } catch {
        if (myId === reqId) status = 'empty';
      }
    }, 300);
    // Cleanup: cancels the pending timer on re-run or unmount.
    return () => clearTimeout(t);
  });

  const messages: Record<Status, string> = {
    empty:     '',
    checking:  '…',
    available: 'Available.',
    taken:     'Already taken.',
    invalid:   'Use 3–30 lowercase letters, numbers, "-", or "_".',
    reserved:  'Reserved — please choose another.',
  };
</script>

<label class="t-label" for={name}>HANDLE</label>
<div class="hp">
  <span class="at" aria-hidden="true">@</span>
  <input
    id={name}
    {name}
    bind:value
    class="input input-mono hp-input"
    autocomplete="username"
    spellcheck="false"
    minlength="3"
    maxlength="30"
    pattern="[a-z0-9_-]+"
    aria-describedby={`${name}-status`}
  />
  <span id={`${name}-status`} class="t-meta hp-status" data-status={status}>
    {messages[status]}
  </span>
</div>

<style>
  .hp { position: relative; }
  .at { position: absolute; left: 12px; top: 9px; color: var(--fg-muted); font-family: var(--font-mono); }
  .hp-input { padding-left: 28px; }
  .hp-status[data-status="available"] { color: var(--success); }
  .hp-status[data-status="taken"],
  .hp-status[data-status="reserved"],
  .hp-status[data-status="invalid"] {
    color: var(--danger);
  }
</style>
