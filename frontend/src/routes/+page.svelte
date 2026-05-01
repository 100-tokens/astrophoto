<script lang="ts">
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import Wordmark from '$lib/components/Wordmark.svelte';

  let { data } = $props();

  let healthLine = $derived(
    data.health ? `backend: ${data.health.status} (db ${data.health.db})` : 'backend: unreachable'
  );
</script>

<AppHeader active="Gallery" />

<main
  style="min-height: calc(100dvh - 64px - 64px); display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 64px 32px; text-align: center;"
>
  <div style="max-width: 640px;">
    <div class="t-eyebrow" style="margin-bottom: 24px; color: var(--accent);">● COMING SOON</div>
    <h1 class="t-display" style="font-size: clamp(40px, 6vw, 88px); margin: 0 0 24px 0;">
      A quiet archive of <em>the night sky</em>
    </h1>
    <p style="font-size: 16px; max-width: 480px; margin: 0 auto 40px;">
      Upload, tag, and share your astrophotographs. A serious home for the work you make.
    </p>
    <Wordmark size={18} italic={true} />
    <div class="t-meta" style="margin-top: 48px;">{healthLine}</div>
  </div>
</main>

<AppFooter />
