<script lang="ts">
  import { cls } from '$lib/utils/cls';

  interface StarDot {
    x: number;
    y: number;
    r: number;
    o: number;
  }

  interface Props {
    target: string;
    src?: string;
    style?: string;
    class?: string;
  }

  let { target, src, style, class: className }: Props = $props();

  // Deterministic hash — matches shared.jsx exactly
  function hash(s: string): number {
    let h = 0;
    for (let i = 0; i < s.length; i++) {
      h = ((h << 5) - h + s.charCodeAt(i)) | 0;
    }
    return Math.abs(h);
  }

  // 6 curated nebula palettes (verbatim from shared.jsx)
  const palettes: [string, string, string][] = [
    // SHO narrowband (gold/teal/red)
    ['rgba(220,140,60,.55)', 'rgba(60,120,160,.45)', 'rgba(180,80,80,.4)'],
    // HOO (blue/cyan)
    ['rgba(80,140,200,.6)', 'rgba(40,180,200,.45)', 'rgba(20,40,80,.5)'],
    // Galaxy (warm core, cool arms)
    ['rgba(255,200,140,.5)', 'rgba(180,140,200,.35)', 'rgba(80,60,120,.4)'],
    // Emission Hα red
    ['rgba(200,80,80,.6)', 'rgba(140,60,100,.4)', 'rgba(60,40,80,.5)'],
    // Lunar / silver
    ['rgba(220,210,200,.7)', 'rgba(140,130,120,.5)', 'rgba(40,40,50,.6)'],
    // Carina cliffs (orange/teal)
    ['rgba(220,160,90,.55)', 'rgba(80,160,170,.4)', 'rgba(60,40,30,.5)']
  ];

  let gradient = $derived.by(() => {
    const h = hash(target);
    const pIdx = h % palettes.length;
    const palette = palettes[pIdx];
    if (!palette) return '';
    const [c1, c2, c3] = palette;
    const x1 = 20 + (h % 50);
    const y1 = 30 + ((h >> 3) % 40);
    const x2 = 50 + ((h >> 5) % 30);
    const y2 = 50 + ((h >> 7) % 30);
    return `radial-gradient(ellipse 60% 45% at ${x1}% ${y1}%, ${c1}, transparent 65%), radial-gradient(ellipse 45% 35% at ${x2}% ${y2}%, ${c2}, transparent 65%), radial-gradient(ellipse 90% 60% at 50% 70%, ${c3}, transparent 75%), #050507`;
  });

  let stars = $derived.by<StarDot[]>(() => {
    const h = hash(target);
    const result: StarDot[] = [];
    for (let i = 0; i < 28; i++) {
      const sh = (h + i * 9301) & 0xfffffff;
      result.push({
        x: sh % 100,
        y: (sh >> 7) % 100,
        r: 0.4 + (((sh >> 13) % 10) / 10) * 1.4,
        o: 0.3 + (((sh >> 17) % 10) / 10) * 0.7
      });
    }
    return result;
  });
</script>

<div class={cls('photo-card', className)} {style}>
  {#if src}
    <img {src} alt={target} />
  {:else}
    <div style="position: absolute; inset: 0; background: {gradient};"></div>
    <svg
      style="position: absolute; inset: 0; width: 100%; height: 100%;"
      preserveAspectRatio="none"
      viewBox="0 0 100 100"
      aria-hidden="true"
    >
      {#each stars as star}
        <circle cx={star.x} cy={star.y} r={star.r * 0.15} fill="white" opacity={star.o} />
      {/each}
    </svg>
  {/if}
</div>
