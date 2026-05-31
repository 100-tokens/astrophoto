<script lang="ts">
  import { untrack } from 'svelte';
  import { invalidateAll } from '$app/navigation';
  import { uploadAvatar, clearAvatar } from '$lib/api/profileClient';
  import { cdn } from '$lib/cdn';

  let {
    avatarId = null,
    displayName = '',
    onChanged
  }: {
    avatarId?: string | null;
    displayName?: string;
    /** Notifies the parent so its in-memory Profile draft stays in sync. */
    onChanged?: (avatarId: string | null) => void;
  } = $props();

  const MAX_BYTES = 5 * 1024 * 1024;
  const ACCEPTED = ['image/jpeg', 'image/png'];

  // Seed once from the prop at mount; thereafter `current` is driven locally
  // by upload/clear. untrack() marks the read intentional (matches
  // IdentitySection's pattern).
  let current = $state(untrack(() => avatarId));
  let busy = $state(false);
  let error = $state<string | null>(null);
  let fileInput: HTMLInputElement | undefined = $state();

  let preview = $derived(current ? cdn(current, { w: 192, h: 192, fit: 'cover' }) : null);
  let initial = $derived((displayName[0] ?? 'U').toUpperCase());

  async function onPick(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = ''; // allow re-selecting the same file after an error
    if (!file) return;
    error = null;
    if (!ACCEPTED.includes(file.type)) {
      error = 'Use a JPEG or PNG image.';
      return;
    }
    if (file.size > MAX_BYTES) {
      error = 'Image must be under 5 MB.';
      return;
    }
    busy = true;
    try {
      const id = await uploadAvatar(fetch, file);
      current = id;
      onChanged?.(id);
    } catch (err) {
      error = (err as Error).message;
      busy = false;
      return;
    }
    busy = false;
    // The avatar is saved and the editor preview already updated. Refresh the
    // header AvatarMenu + profile hero in the BACKGROUND — never block the
    // button on it, so a slow (or previously looping) load can't strand it in
    // "Uploading…".
    invalidateAll().catch(() => {});
  }

  async function onRemove() {
    busy = true;
    error = null;
    try {
      await clearAvatar(fetch);
      current = null;
      onChanged?.(null);
    } catch (err) {
      error = (err as Error).message;
      busy = false;
      return;
    }
    busy = false;
    invalidateAll().catch(() => {});
  }
</script>

<fieldset class="section">
  <legend>Avatar</legend>
  <div class="row">
    {#if preview}
      <img class="thumb" src={preview} alt="Current avatar" width="72" height="72" />
    {:else}
      <div class="thumb thumb--initial" aria-hidden="true">{initial}</div>
    {/if}
    <div class="controls">
      <input
        bind:this={fileInput}
        type="file"
        accept="image/jpeg,image/png"
        onchange={onPick}
        disabled={busy}
        hidden
      />
      <button type="button" class="btn" onclick={() => fileInput?.click()} disabled={busy}>
        {busy ? 'Uploading…' : current ? 'Change photo' : 'Upload photo'}
      </button>
      {#if current}
        <button
          type="button"
          class="btn btn--ghost"
          onclick={() => void onRemove()}
          disabled={busy}
        >
          Remove
        </button>
      {/if}
    </div>
  </div>
  {#if error}<p class="err">{error}</p>{/if}
  <p class="hint">JPEG or PNG, up to 5 MB. Cropped to a square.</p>
</fieldset>

<style>
  .section {
    border: 1px solid var(--border-subtle);
    padding: 16px;
    margin: 0 0 16px;
  }
  legend {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-muted);
    padding: 0 6px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .thumb {
    width: 72px;
    height: 72px;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
    background: var(--bg-canvas);
  }
  .thumb--initial {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent);
    color: var(--accent-ink);
    font-family: var(--font-display, serif);
    font-size: 32px;
  }
  .controls {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .btn {
    background: var(--bg-canvas);
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 8px 12px;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    cursor: pointer;
  }
  .btn:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .btn--ghost {
    background: transparent;
    color: var(--fg-muted);
  }
  .err {
    color: var(--danger, #c33);
    font-family: var(--font-mono);
    font-size: 11px;
    margin: 10px 0 0;
  }
  .hint {
    color: var(--fg-muted);
    font-family: var(--font-mono);
    font-size: 11px;
    margin: 10px 0 0;
  }
</style>
