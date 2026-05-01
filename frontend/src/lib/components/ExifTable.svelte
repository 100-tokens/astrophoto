<script lang="ts">
  import { cls } from '$lib/utils/cls';

  interface ExifRow {
    label: string;
    value: string;
    sublabel?: string;
    valueAccent?: boolean;
    sublabelAccent?: boolean;
  }

  interface Props {
    rows: ExifRow[];
    class?: string;
  }

  let { rows, class: className }: Props = $props();
</script>

<table class={cls('exif', className)}>
  <tbody>
    {#each rows as row}
      <tr>
        <th scope="row">{row.label}</th>
        <td style={row.valueAccent ? 'color: var(--accent);' : ''}>
          {row.value}
          {#if row.sublabel}
            <br /><span
              style="color: {row.sublabelAccent
                ? 'var(--accent)'
                : 'var(--fg-muted)'}; font-size: var(--t-xs);">{row.sublabel}</span
            >
          {/if}
        </td>
      </tr>
    {/each}
  </tbody>
</table>
