<script lang="ts">
  import { enhance } from '$app/forms';
  import SetupForm from '$lib/components/SetupForm.svelte';
  import type { PageProps } from './$types';

  let { form }: PageProps = $props();
  let submitting = $state(false);
</script>

<header class="header">
  <h1>New equipment setup</h1>
</header>

{#if form?.error}
  <p class="form-error">{form.error}</p>
{/if}

<form
  method="POST"
  class:submitting
  use:enhance={() => {
    submitting = true;
    return async ({ update }) => {
      await update();
      submitting = false;
    };
  }}
>
  <SetupForm
    initial={null}
    cancelHref="/settings/equipment"
    submitLabel={submitting ? 'Saving…' : 'Create setup'}
  />
</form>

<style>
  .header {
    margin-bottom: 1rem;
  }
  .submitting {
    opacity: 0.6;
    pointer-events: none;
  }
  .form-error {
    color: var(--error, #c00);
    margin-bottom: 0.5rem;
  }
</style>
