<script lang="ts">
  import { cls } from '$lib/utils/cls';

  interface Props {
    size?: number;
    color?: string;
    class?: string;
  }

  let { size = 28, color = 'currentColor', class: className }: Props = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 64 64"
  fill="none"
  stroke={color}
  stroke-width="1.4"
  stroke-linecap="square"
  class={cls('mark-reticle', className)}
  aria-hidden="true"
>
  <circle cx="32" cy="32" r="22" stroke-width="1.6" />
  <circle cx="32" cy="32" r="14" stroke-dasharray="2.5 2.5" stroke-width="1" opacity="0.7" />
  <line x1="32" y1="2" x2="32" y2="18" stroke-width="1.6" />
  <line x1="32" y1="46" x2="32" y2="62" stroke-width="1.6" />
  <line x1="2" y1="32" x2="18" y2="32" stroke-width="1.6" />
  <line x1="46" y1="32" x2="62" y2="32" stroke-width="1.6" />
  <circle cx="32" cy="32" r="2.4" fill={color} stroke="none" />
  <circle cx="24" cy="26" r="1.4" fill={color} stroke="none" />
  <circle cx="40" cy="36" r="1.7" fill={color} stroke="none" />
  <circle cx="36" cy="22" r="1.1" fill={color} stroke="none" />
  <line x1="24" y1="26" x2="36" y2="22" stroke-width="0.8" />
  <line x1="36" y1="22" x2="40" y2="36" stroke-width="0.8" />
</svg>
