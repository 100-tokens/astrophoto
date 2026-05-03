<script lang="ts">
  let { onEdit }: { onEdit: () => void } = $props();
</script>

<div class="banner" role="status">
  <span class="dot">●</span>
  <span class="label">VIEWING YOUR OWN PROFILE · OWNER MODE</span>
  <button type="button" class="btn-edit" onclick={onEdit}>Edit profile</button>
</div>

<style>
  .banner {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 24px;
    background: color-mix(in srgb, var(--accent) 12%, var(--bg-canvas));
    color: var(--fg-primary);
    border-bottom: 1px solid var(--border-subtle);
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.06em;
  }
  .dot {
    color: var(--accent);
  }
  .label {
    flex: 1;
  }
  .btn-edit {
    background: var(--accent);
    color: var(--accent-ink);
    border: 0;
    padding: 6px 12px;
    font-family: var(--font-mono);
    font-size: 11px;
    cursor: pointer;
  }
</style>
