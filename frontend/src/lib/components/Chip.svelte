<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cls } from '$lib/utils/cls';

  interface Props {
    accent?: boolean;
    class?: string;
    children?: Snippet;
  }

  let { accent = false, class: className, children }: Props = $props();
</script>

<!-- Chip is always a <span> — use Button with variant="ghost" for interactive chips -->
<span class={cls('chip', accent && 'chip-accent', className)}>
  {@render children?.()}
</span>
