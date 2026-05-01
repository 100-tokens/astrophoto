<script lang="ts">
  import { cls } from '$lib/utils/cls';

  interface Props {
    text?: string;
    size?: number;
    italic?: boolean;
    weight?: number;
    class?: string;
  }

  let {
    text = 'Astrophoto',
    size = 22,
    italic = false,
    weight = 600,
    class: className
  }: Props = $props();
</script>

<span
  class={cls('wordmark', className)}
  style="font-family: var(--font-display); font-size: {size}px; font-weight: {weight}; font-style: {italic
    ? 'italic'
    : 'normal'}; letter-spacing: -0.01em; line-height: 1;"
>
  {text}
</span>
