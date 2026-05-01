<script lang="ts">
  import { cls } from '$lib/utils/cls';

  interface Props {
    size?: number;
    color?: string;
    inset?: number;
    class?: string;
  }

  let { size = 14, color = 'var(--accent)', inset = -8, class: className }: Props = $props();

  interface Corner {
    top?: number;
    bottom?: number;
    left?: number;
    right?: number;
    bTop: boolean;
    bRight: boolean;
    bBottom: boolean;
    bLeft: boolean;
  }

  let corners = $derived<Corner[]>([
    { top: inset, left: inset, bTop: true, bLeft: true, bBottom: false, bRight: false },
    { top: inset, right: inset, bTop: true, bRight: true, bBottom: false, bLeft: false },
    { bottom: inset, right: inset, bBottom: true, bRight: true, bTop: false, bLeft: false },
    { bottom: inset, left: inset, bBottom: true, bLeft: true, bTop: false, bRight: false }
  ]);
</script>

<div
  class={cls('corner-marks-wrap', className)}
  style="position: absolute; inset: 0; pointer-events: none;"
>
  {#each corners as corner}
    <div
      style="
        position: absolute;
        width: {size}px;
        height: {size}px;
        pointer-events: none;
        {corner.top !== undefined ? `top: ${corner.top}px;` : ''}
        {corner.bottom !== undefined ? `bottom: ${corner.bottom}px;` : ''}
        {corner.left !== undefined ? `left: ${corner.left}px;` : ''}
        {corner.right !== undefined ? `right: ${corner.right}px;` : ''}
        border-color: {color};
        border-style: solid;
        border-top-width: {corner.bTop ? '1px' : '0'};
        border-right-width: {corner.bRight ? '1px' : '0'};
        border-bottom-width: {corner.bBottom ? '1px' : '0'};
        border-left-width: {corner.bLeft ? '1px' : '0'};
      "
    ></div>
  {/each}
</div>
