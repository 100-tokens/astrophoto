<script lang="ts">
  let {
    tagline,
    isOwner,
    onEditProfile
  }: {
    tagline: string | null | undefined;
    isOwner: boolean;
    onEditProfile: () => void;
  } = $props();
</script>

{#if tagline}
  <p class="tag">{tagline}</p>
{:else if isOwner}
  <button type="button" class="tag-prompt" onclick={onEditProfile}>Add a tagline</button>
{/if}

<style>
  .tag {
    margin: 8px 0 0;
    font-size: 16px;
    color: var(--fg-secondary);
    max-width: 640px;
  }
  .tag-prompt {
    background: transparent;
    color: var(--accent);
    border: 0;
    padding: 4px 0;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
  }
</style>
