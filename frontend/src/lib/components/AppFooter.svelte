<script lang="ts">
  import { cls } from '$lib/utils/cls';

  interface Props {
    class?: string;
  }

  let { class: className }: Props = $props();
</script>

<footer
  class={cls(className)}
  style="border-top: 1px solid var(--border-subtle); padding: 32px; display: flex; justify-content: space-between; font-family: var(--font-mono); font-size: 11px; letter-spacing: 0.08em; text-transform: uppercase; color: var(--fg-muted);"
>
  <span>Astrophoto · Est. MMXXVI</span>
  <span style="display: flex; gap: 24px;">
    <a href="/about">About</a>
    <a href="/terms">Terms</a>
    <a href="/privacy">Privacy</a>
    <a href="/contact">Contact</a>
    <a href="/rss.xml">RSS</a>
  </span>
  <span>52°31′N · 13°24′E · Bortle 8</span>
</footer>
