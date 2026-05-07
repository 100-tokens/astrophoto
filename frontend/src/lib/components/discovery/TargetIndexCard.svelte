<script lang="ts">
  import type { TargetListItem } from '$lib/api/TargetListItem';
  import { objectTypeLabel, constellationLabel } from '$lib/data/celestial';
  import { cdn } from '$lib/cdn';

  interface Props {
    target: TargetListItem;
  }
  let { target }: Props = $props();
</script>

<a class="card" href="/t/{target.slug}">
  <div class="thumbs">
    {#each Array(3) as _, i (i)}
      {@const thumb = target.preview_thumbs[i]}
      <div class="thumb" class:placeholder={!thumb}>
        {#if thumb}
          <img loading="lazy" alt="" src={cdn(thumb.photo_id, { w: 240, h: 240, fit: 'cover' })} />
        {/if}
      </div>
    {/each}
  </div>
  <h3>{target.slug.toUpperCase()}</h3>
  <p class="canonical">{target.canonical_name}</p>
  <p class="meta">
    {objectTypeLabel(target.object_type)}
    {#if target.constellation}
      · {constellationLabel(target.constellation)}{/if}
  </p>
  <p class="count">{String(target.photo_count)} photos</p>
</a>

<style>
  .card {
    display: block;
    color: inherit;
    text-decoration: none;
    padding: 0.75rem;
    border: 1px solid var(--border-subtle, #ddd);
    border-radius: var(--r-md, 6px);
    background: var(--bg-elevated, #fff);
    transition: border-color 0.15s;
  }
  .card:hover {
    border-color: var(--accent, #4a90e2);
  }
  .thumbs {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 2px;
    margin-bottom: 0.5rem;
    aspect-ratio: 3 / 1;
  }
  .thumb {
    background: var(--bg-elevated, #2a2a2a);
    overflow: hidden;
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .thumb.placeholder {
    /* Diagonal-stripe pattern so the empty preview reads as "no photo here yet"
       rather than an invisible card-color rectangle. Subtle, doesn't fight the
       layout when real thumbs land. */
    background-color: var(--bg-faint, #141414);
    background-image: repeating-linear-gradient(
      135deg,
      transparent 0,
      transparent 8px,
      rgba(255, 255, 255, 0.03) 8px,
      rgba(255, 255, 255, 0.03) 9px
    );
    border: 1px solid var(--border-subtle, #2a2a2a);
  }
  h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    font-family: var(--font-mono);
  }
  .canonical {
    margin: 0.25rem 0 0;
    font-size: 0.95rem;
  }
  .meta {
    margin: 0.25rem 0 0;
    font-size: 0.85rem;
    color: var(--fg-muted, #888);
  }
  .count {
    margin: 0.5rem 0 0;
    font-size: 0.8rem;
    color: var(--fg-secondary, #aaa);
    font-family: var(--font-mono);
  }
</style>
