<script lang="ts">
  import { untrack } from 'svelte';
  import { invalidateAll } from '$app/navigation';

  // Current handle from the layout's `user`. Renaming is a deliberate
  // action (not autosaved like display_name): it can collide, it has format
  // rules, and it creates a 301 redirect from the old handle — so it gets an
  // explicit Save button, a live availability check, and clear errors.
  let { current }: { current: string } = $props();

  // Independent editable copy — seeded once from the prop (untrack dodges
  // the "$state from prop" lint). After a successful save it already equals
  // the new handle, so no re-sync is needed.
  let value = $state(untrack(() => current));
  type Avail = 'current' | 'checking' | 'available' | 'taken' | 'reserved' | 'invalid' | 'unknown';
  let avail = $state<Avail>('current');
  let status = $state<'idle' | 'saving' | 'saved' | 'error'>('idle');
  let checkTimer: ReturnType<typeof setTimeout> | undefined;

  // Handles are lowercased + restricted to [a-z0-9_-], 3–30 chars.
  let normalized = $derived(value.trim().toLowerCase());
  let dirty = $derived(normalized !== current.toLowerCase());
  let canSave = $derived(dirty && avail === 'available' && status !== 'saving');

  function onInput(e: Event) {
    value = (e.target as HTMLInputElement).value;
    status = 'idle';
    if (checkTimer) clearTimeout(checkTimer);
    const h = value.trim().toLowerCase();
    if (h === current.toLowerCase()) {
      avail = 'current';
      return;
    }
    if (!/^[a-z0-9_-]{3,30}$/.test(h)) {
      avail = 'invalid';
      return;
    }
    avail = 'checking';
    checkTimer = setTimeout(async () => {
      try {
        const r = await fetch(`/api/auth/handle-check?handle=${encodeURIComponent(h)}`, {
          credentials: 'include'
        });
        if (!r.ok) {
          avail = 'unknown';
          return;
        }
        const j = (await r.json()) as { status: string };
        avail =
          (
            {
              available: 'available',
              taken: 'taken',
              reserved: 'reserved',
              invalid: 'invalid'
            } as const
          )[j.status] ?? 'unknown';
      } catch {
        avail = 'unknown';
      }
    }, 350);
  }

  async function save() {
    if (!canSave) return;
    status = 'saving';
    try {
      const r = await fetch('/api/me/handle', {
        method: 'POST',
        credentials: 'include',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ handle: normalized })
      });
      if (r.ok || r.status === 204) {
        status = 'saved';
        avail = 'current';
        // Refresh layout/page data so the new handle propagates to the
        // header, profile links, and permalinks everywhere.
        await invalidateAll();
      } else if (r.status === 409) {
        avail = 'taken';
        status = 'idle';
      } else if (r.status === 422) {
        avail = 'invalid';
        status = 'idle';
      } else {
        status = 'error';
      }
    } catch {
      status = 'error';
    }
  }

  $effect(() => () => clearTimeout(checkTimer));
</script>

<div class="handle">
  <div class="input-wrap">
    <span class="at">@</span>
    <input
      class="input"
      {value}
      oninput={onInput}
      aria-label="Handle"
      autocapitalize="none"
      autocorrect="off"
      spellcheck="false"
      maxlength="30"
    />
  </div>
  <button type="button" class="btn btn-secondary btn-sm" onclick={save} disabled={!canSave}>
    {status === 'saving' ? 'Saving…' : 'Save handle'}
  </button>

  {#if status === 'saved'}
    <span class="msg ok">● Saved — your profile is now /u/{normalized}</span>
  {:else if avail === 'checking'}
    <span class="msg muted">checking…</span>
  {:else if avail === 'available'}
    <span class="msg ok">● available</span>
  {:else if avail === 'taken'}
    <span class="msg err">● already taken</span>
  {:else if avail === 'reserved'}
    <span class="msg err">● reserved</span>
  {:else if avail === 'invalid'}
    <span class="msg err">● 3–30 chars, lowercase letters / digits / - _</span>
  {:else if status === 'error'}
    <span class="msg err">● could not save — retry</span>
  {/if}
</div>

<style>
  .handle {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .input-wrap {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .at {
    font-family: var(--font-mono);
    color: var(--fg-muted);
  }
  .msg {
    font-family: var(--font-mono);
    font-size: 11px;
  }
  .msg.ok {
    color: var(--accent);
  }
  .msg.err {
    color: var(--danger);
  }
  .msg.muted {
    color: var(--fg-faint);
  }
</style>
