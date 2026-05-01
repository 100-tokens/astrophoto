<script lang="ts">
  import { cls } from '$lib/utils/cls';
  import MarkReticle from './MarkReticle.svelte';
  import Wordmark from './Wordmark.svelte';

  interface Props {
    size?: number;
    class?: string;
  }

  let { size = 28, class: className }: Props = $props();

  // Scale wordmark proportionally to mark size (default mark=28, wordmark=22)
  let wordmarkSize = $derived(Math.round(size * (22 / 28)));
</script>

<div class={cls('logo', className)} style="display: flex; align-items: center; gap: 12px;">
  <MarkReticle {size} color="var(--accent)" />
  <Wordmark size={wordmarkSize} />
</div>
