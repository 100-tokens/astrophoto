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

  let savedAt: number | null = $state(null);
  let error = $state(false);
  let timer: number | undefined;

  async function save() {
    const fd = new FormData();
    fd.set(name, value);
    try {
      const r = await fetch(action, { method: 'POST', body: fd });
      if (!r.ok) throw new Error(String(r.status));
      error = false;
      savedAt = Date.now();
    } catch {
      error = true;
    }
  }

  function onInput(e: Event) {
    value = (e.target as HTMLInputElement).value;
    if (timer) clearTimeout(timer);
    timer = window.setTimeout(save, 600);
  }

  function showSaved() {
    return savedAt && Date.now() - savedAt < 2000;
  }
</script>

<div class="autosave">
  <input {type} {name} {value} oninput={onInput} class="input" />
  {#if showSaved()}<span class="saved">● Saved</span>{/if}
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
