<script lang="ts">
  import type { SocialLink } from '$lib/api/SocialLink';
  import type { SocialPlatform } from '$lib/api/SocialPlatform';

  let { links }: { links: SocialLink[] } = $props();

  function label(p: SocialPlatform): string {
    switch (p) {
      case 'twitter':
        return '𝕏';
      case 'instagram':
        return 'IG';
      case 'bluesky':
        return 'BS';
      case 'astrobin':
        return 'AB';
      case 'mastodon':
        return 'MS';
      case 'youtube':
        return 'YT';
      case 'website':
        return '🌐';
    }
  }
</script>

{#if links.length > 0}
  <ul class="socials" aria-label="Social links">
    {#each links as link}
      <li>
        <a href={link.url} target="_blank" rel="noopener nofollow">{label(link.platform)}</a>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .socials {
    list-style: none;
    padding: 0;
    margin: 12px 0 0;
    display: flex;
    gap: 16px;
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .socials a {
    color: var(--fg-secondary);
    text-decoration: none;
  }
  .socials a:hover {
    color: var(--accent);
  }
</style>
