<script lang="ts">
  import { cdn, srcset, type Transform } from '$lib/cdn';

  interface Props {
    photoId: string;
    alt: string;
    w?: number;
    transform?: Omit<Transform, 'w'>;
    sizes?: string;
    blurhash?: string;
    aspectRatio?: string; // e.g. "3/2"
    class?: string;
  }

  let {
    photoId,
    alt,
    w = 800,
    transform = {},
    sizes = '(max-width: 640px) 100vw, 800px',
    blurhash,
    aspectRatio,
    class: cls = '',
  }: Props = $props();

  const widths = [w, w * 2, w * 3];

  // Decode blurhash to a CSS gradient placeholder if provided.
  // Lightweight approach: skip decoding here, render solid bg.
  // (P2 task adds the @ts/blurhash decoder for richer placeholders.)
</script>

<img
  src={cdn(photoId, { ...transform, w })}
  srcset={srcset(photoId, widths, transform)}
  {sizes}
  {alt}
  loading="lazy"
  decoding="async"
  style:aspect-ratio={aspectRatio}
  class={cls}
  data-blurhash={blurhash ?? ''}
/>
