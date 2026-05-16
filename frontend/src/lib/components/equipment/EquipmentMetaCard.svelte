<script lang="ts">
  import type { EquipmentItemDetail } from '$lib/api/EquipmentItemDetail';

  let { item }: { item: EquipmentItemDetail } = $props();

  function fmtDate(iso: string | null | undefined): string {
    if (!iso) return '—';
    return new Date(iso).toISOString().slice(0, 10);
  }
</script>

<aside class="meta-card">
  <div class="t-label">CATALOG ITEM</div>
  <dl>
    <div><dt>STATUS</dt><dd class="status status-{item.status}">● {item.status}</dd></div>
    <div><dt>CANONICAL</dt><dd class="mono">{item.canonical_name}</dd></div>
    <div><dt>CREATED</dt><dd class="mono">{fmtDate(item.created_at)}</dd></div>
    <div><dt>APPROVED</dt><dd class="mono">{fmtDate(item.approved_at)}</dd></div>
    <div><dt>SUBMITTED BY</dt><dd class="mono">{item.submitted_by ?? '—'}</dd></div>
  </dl>
</aside>

<style>
  .meta-card {
    padding: 20px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-raised);
    margin: 0 64px 48px;
    max-width: 320px;
    margin-left: auto;
  }
  dl {
    margin: 16px 0 0;
    display: grid;
    grid-template-columns: 1fr;
    gap: 12px;
  }
  dl > div {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 16px;
  }
  dt {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.08em;
    color: var(--fg-muted);
  }
  dd {
    margin: 0;
    font-size: 12px;
    color: var(--fg-secondary);
    text-align: right;
  }
  dd.mono { font-family: var(--font-mono); }
  .status { font-family: var(--font-mono); font-size: 11px; }
  .status-approved { color: var(--success); }
  .status-pending  { color: var(--warning); }
  .status-rejected { color: var(--danger); }
  .status-merged   { color: var(--fg-muted); }
</style>
