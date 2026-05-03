<script lang="ts">
  let {
    handle,
    displayName
  }: {
    handle: string;
    displayName: string;
  } = $props();

  let initial = $derived((displayName[0] ?? handle[0] ?? 'U').toUpperCase());
</script>

<div class="avatar" aria-hidden="true">{initial}</div>

<style>
  .avatar {
    width: 144px;
    height: 144px;
    background: var(--accent);
    color: var(--accent-ink);
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-display, 'Source Serif 4', serif);
    font-size: 64px;
    border: 4px solid var(--bg-canvas);
  }
  @media (max-width: 640px) {
    .avatar {
      width: 96px;
      height: 96px;
      font-size: 44px;
    }
  }
</style>
