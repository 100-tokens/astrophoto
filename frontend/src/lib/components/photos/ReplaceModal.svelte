<script lang="ts">
  import Modal from '../Modal.svelte';
  import Button from '../Button.svelte';

  let { open = $bindable(false), photoId, onreplaced }: {
    open: boolean; photoId: string; onreplaced: () => void;
  } = $props();

  let file = $state<File | null>(null);
  let busy = $state(false);
  let err = $state<string | null>(null);

  async function submit() {
    if (!file) { err = 'Choose a file.'; return; }
    busy = true; err = null;
    const fd = new FormData();
    fd.append('file', file, file.name);
    const r = await fetch(`/api/photos/${photoId}/replace`, {
      method: 'POST', body: fd, credentials: 'include'
    });
    busy = false;
    if (!r.ok) { err = `Replace failed: ${await r.text()}`; return; }
    open = false; file = null;
    onreplaced();
  }
</script>

<Modal bind:open title="Replace image">
  <p class="t-meta">Caption, target, comments and appreciations are kept. Pipeline reprocesses thumbnails.</p>
  <input type="file" accept="image/jpeg,image/png,image/tiff"
    onchange={(e) => (file = (e.target as HTMLInputElement).files?.[0] ?? null)} />
  {#if err}<p class="t-meta form-error">{err}</p>{/if}
  <div class="actions">
    <Button variant="ghost" onclick={() => (open = false)}>Cancel</Button>
    <Button variant="primary" disabled={!file || busy} onclick={submit}>
      {busy ? 'Uploading…' : 'Replace'}
    </Button>
  </div>
</Modal>

<style>
  .form-error { color: var(--danger); }
  .actions { display: flex; gap: 12px; justify-content: flex-end; margin-top: 16px; }
</style>
