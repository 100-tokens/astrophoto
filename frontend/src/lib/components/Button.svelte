<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cls } from '$lib/utils/cls';

  interface Props {
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
    size?: 'sm' | 'default' | 'lg';
    type?: 'button' | 'submit' | 'reset';
    disabled?: boolean;
    href?: string;
    class?: string;
    onclick?: (e: MouseEvent) => void;
    children?: Snippet;
  }

  let {
    variant = 'primary',
    size = 'default',
    type = 'button',
    disabled = false,
    href,
    class: className,
    onclick,
    children
  }: Props = $props();

  let btnClass = $derived(
    cls('btn', `btn-${variant}`, size !== 'default' && `btn-${size}`, className)
  );
</script>

{#if href}
  <a {href} class={btnClass} aria-disabled={disabled}>
    {@render children?.()}
  </a>
{:else}
  <button class={btnClass} {type} {disabled} {onclick}>
    {@render children?.()}
  </button>
{/if}
