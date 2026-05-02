<script lang="ts">
  let {
    value = $bindable(''),
    name,
    action,
    type = 'text'
  }: {
    value: string;
    name: string;
    action: string;
    type?: string;
  } = $props();

  let saved = $state(false);
  let error = $state(false);
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;
  let savedTimer: ReturnType<typeof setTimeout> | undefined;

  async function save() {
    const fd = new FormData();
    fd.set(name, value);
    try {
      const r = await fetch(action, { method: 'POST', body: fd });
      if (!r.ok) throw new Error(String(r.status));
      error = false;
      saved = true;
      if (savedTimer) clearTimeout(savedTimer);
      savedTimer = setTimeout(() => {
        saved = false;
      }, 2000);
    } catch {
      error = true;
    }
  }

  function onInput(e: Event) {
    value = (e.target as HTMLInputElement).value;
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(save, 600);
  }

  // Clear pending timers on unmount so a stale save doesn't fire after the
  // component is gone (and the savedTimer doesn't try to mutate dead state).
  $effect(() => {
    return () => {
      if (debounceTimer) clearTimeout(debounceTimer);
      if (savedTimer) clearTimeout(savedTimer);
    };
  });
</script>

<div class="autosave">
  <input {type} {name} {value} oninput={onInput} class="input" />
  {#if saved}<span class="saved">● Saved</span>{/if}
  {#if error}<span class="err">● Save failed — retry</span>{/if}
</div>

<style>
  .autosave {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .saved {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--accent);
  }
  .err {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--danger);
  }
</style>
