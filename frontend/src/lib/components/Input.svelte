<script lang="ts">
  import { cls } from '$lib/utils/cls';

  interface Props {
    value?: string;
    placeholder?: string;
    mono?: boolean;
    type?: string;
    name?: string;
    id?: string;
    required?: boolean;
    class?: string;
  }

  let {
    value = $bindable(''),
    placeholder,
    mono = false,
    type = 'text',
    name,
    id,
    required,
    class: className
  }: Props = $props();
</script>

<input
  class={cls('input', mono && 'input-mono', className)}
  {type}
  {name}
  {id}
  {placeholder}
  {required}
  bind:value
/>
