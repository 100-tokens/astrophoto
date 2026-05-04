<script lang="ts">
  import { goto } from '$app/navigation';
  import SetupForm from '$lib/components/SetupForm.svelte';
  import type { SetupDetail } from '$lib/api/SetupDetail';
  import type { SetupInput } from '$lib/api/SetupInput';

  let { data }: { data: { setup: SetupDetail } } = $props();

  const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '';

  let error = $state<string | null>(null);
  let saving = $state(false);

  async function onsubmit(input: SetupInput) {
    saving = true;
    error = null;
    const r = await fetch(`${API}/api/equipment/setups/${data.setup.id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify(input)
    });
    saving = false;
    if (r.status === 200) {
      await goto('/settings/equipment');
      return;
    }
    if (r.status === 422) {
      try {
        const body = await r.json();
        error = typeof body?.error === 'string' ? body.error : 'Validation error';
      } catch {
        error = 'Validation error';
      }
      return;
    }
    if (r.status === 404) {
      error = 'Setup no longer exists';
      return;
    }
    error = `Backend error (${r.status})`;
  }

  function oncancel() {
    goto('/settings/equipment');
  }
</script>

<header class="header">
  <h1>Edit setup: {data.setup.name}</h1>
</header>

{#if error}
  <p class="form-error">{error}</p>
{/if}

<div class:saving>
  <SetupForm
    initial={data.setup}
    submitLabel={saving ? 'Saving…' : 'Save changes'}
    {onsubmit}
    {oncancel}
  />
</div>

<style>
  .header {
    margin-bottom: 1rem;
  }
  .saving {
    opacity: 0.6;
    pointer-events: none;
  }
  .form-error {
    color: var(--error, #c00);
    margin-bottom: 0.5rem;
  }
</style>
