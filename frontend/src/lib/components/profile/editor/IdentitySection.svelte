<script lang="ts">
  let {
    displayName = '',
    tagline = null,
    onCommit
  }: {
    displayName?: string;
    tagline?: string | null;
    onCommit: (patch: { display_name?: string; tagline?: string | null }) => Promise<void>;
  } = $props();

  let localName = $state(displayName);
  let localTag = $state(tagline ?? '');
  let savedName = $state(displayName);
  let savedTag = $state(tagline ?? '');

  async function commitName() {
    if (localName === savedName) return;
    await onCommit({ display_name: localName });
    savedName = localName;
  }

  async function commitTag() {
    const norm = localTag.trim() === '' ? null : localTag.trim();
    const normSaved = savedTag.trim() === '' ? null : savedTag.trim();
    if (norm === normSaved) return;
    await onCommit({ tagline: norm });
    savedTag = localTag;
  }
</script>

<fieldset class="section">
  <legend>Identity</legend>
  <label class="field">
    <span>Display name</span>
    <input type="text" bind:value={localName} onblur={() => void commitName()} maxlength="60" />
  </label>
  <label class="field">
    <span>Tagline</span>
    <input type="text" bind:value={localTag} onblur={() => void commitTag()} maxlength="140" />
  </label>
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
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 12px;
  }
  .field span {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
  }
  .field input {
    background: var(--bg-canvas);
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 8px 10px;
    font-size: 14px;
  }
</style>
