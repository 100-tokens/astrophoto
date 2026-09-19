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
    minlength?: number;
    step?: string | number;
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
    minlength,
    step,
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
  {minlength}
  {step}
  bind:value
/>
