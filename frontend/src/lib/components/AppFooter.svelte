<script lang="ts">
  import { cls } from '$lib/utils/cls';

  interface Props {
    class?: string;
  }

  let { class: className }: Props = $props();
</script>

<footer class={cls('app-footer', className)}>
  <span class="app-footer-mark">Astrophoto · Est. MMXXVI</span>
  <span class="app-footer-links">
    <a href="/about">About</a>
    <a href="/terms">Terms</a>
    <a href="/privacy">Privacy</a>
    <a href="/contact">Contact</a>
    <a href="/rss.xml">RSS</a>
  </span>
  <span class="app-footer-geo">52°31′N · 13°24′E · Bortle 8</span>
</footer>

<style>
  .app-footer {
    border-top: 1px solid var(--border-subtle);
    padding: 32px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--fg-muted);
  }
  .app-footer-links {
    display: flex;
    gap: 24px;
    flex-wrap: wrap;
    justify-content: center;
  }
  /* Stack the three groups vertically on narrow viewports — at 390 px the
     space-between flex collapses the wordmark and link cluster into each
     other, leaving "ASTROPHOTOABOUT TERMS …" with no separation. */
  @media (max-width: 720px) {
    .app-footer {
      flex-direction: column;
      gap: 12px;
      padding: 24px 24px 32px;
      text-align: center;
    }
    .app-footer-links {
      gap: 16px;
    }
  }
</style>
