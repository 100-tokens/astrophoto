<script lang="ts">
  import { cls } from '$lib/utils/cls';

  interface Props {
    value?: string;
    placeholder?: string;
    rows?: number;
    name?: string;
    id?: string;
    required?: boolean;
    class?: string;
  }

  let {
    value = $bindable(''),
    placeholder,
    rows,
    name,
    id,
    required,
    class: className
  }: Props = $props();
</script>

<textarea class={cls('textarea', className)} {rows} {name} {id} {placeholder} {required} bind:value
></textarea>
