<script lang="ts">
  let {
    bio,
    isOwner,
    onEditProfile
  }: {
    bio: string | null | undefined;
    isOwner: boolean;
    onEditProfile: () => void;
  } = $props();

  let expanded = $state(false);
</script>

{#if bio}
  <section class="about">
    <h2 class="about-label">ABOUT</h2>
    <!-- bio_html is server-sanitised — this {@html} is intentional. -->
    <div class="bio" class:clamped={!expanded}>
      {@html bio}
    </div>
    <button type="button" class="more" onclick={() => (expanded = !expanded)}>
      {expanded ? 'less ↑' : 'more ↓'}
    </button>
  </section>
{:else if isOwner}
  <section class="about empty">
    <button type="button" class="prompt" onclick={onEditProfile}>
      Tell visitors about your astrophotography
    </button>
  </section>
{/if}

<style>
  .about {
    padding: 24px 32px;
    border-top: 1px solid var(--border-subtle);
  }
  .about-label {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
    margin: 0 0 8px;
    letter-spacing: 0.06em;
  }
  .bio {
    color: var(--fg-secondary);
    max-width: 640px;
    line-height: 1.55;
  }
  .bio.clamped {
    display: -webkit-box;
    -webkit-line-clamp: 4;
    line-clamp: 4;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .more {
    margin-top: 8px;
    background: none;
    border: 0;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: 11px;
    cursor: pointer;
    padding: 0;
  }
  .empty .prompt {
    background: transparent;
    color: var(--accent);
    border: 1px dashed var(--border-subtle);
    padding: 16px 20px;
    font-family: var(--font-mono);
    font-size: 12px;
    cursor: pointer;
    width: 100%;
  }
</style>
