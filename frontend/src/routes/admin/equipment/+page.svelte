<script lang="ts">
  import { untrack } from 'svelte';
  import { goto, invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import { deleteEquipment } from '$lib/api/adminClient';
  import type { AdminEquipmentItem } from '$lib/api/AdminEquipmentItem';

  let { data } = $props();

  const KINDS = ['telescope', 'camera', 'mount', 'filter', 'focal_modifier', 'guiding'];

  // Filter inputs seed from the loaded query once; navigation re-runs the load.
  let kind = $state(untrack(() => data.kind));
  let q = $state(untrack(() => data.q));
  let busy = $state(false);
  let errorMsg = $state<string | null>(null);

  function applyFilters() {
    const params = new URLSearchParams();
    if (kind) params.set('kind', kind);
    if (q.trim()) params.set('q', q.trim());
    void goto(`/admin/equipment${params.toString() ? `?${params}` : ''}`);
  }

  async function remove(item: AdminEquipmentItem) {
    if (!confirm(`Delete "${item.display_name}"? This cannot be undone.`)) return;
    busy = true;
    errorMsg = null;
    try {
      await deleteEquipment(fetch, item.id);
      await invalidateAll();
    } catch (e) {
      errorMsg =
        (e as Error).message === 'in_use'
          ? `"${item.display_name}" is still used by photos or setups — cannot delete.`
          : (e as Error).message;
    } finally {
      busy = false;
    }
  }

  function gotoPage(p: number) {
    const params = new URLSearchParams(page.url.searchParams);
    params.set('page', String(p));
    void goto(`/admin/equipment?${params}`);
  }
</script>

<svelte:head><title>Equipment · Admin · Astrophoto</title></svelte:head>

<header class="head">
  <h1>Equipment catalog</h1>
  <p class="count">{data.total} item{data.total === 1 ? '' : 's'}</p>
</header>

<div class="filters">
  <select bind:value={kind} onchange={applyFilters}>
    <option value="">All kinds</option>
    {#each KINDS as k}
      <option value={k}>{k}</option>
    {/each}
  </select>
  <input
    type="search"
    placeholder="Search brand / model / name…"
    bind:value={q}
    onkeydown={(e) => e.key === 'Enter' && applyFilters()}
  />
  <button type="button" onclick={applyFilters}>Search</button>
</div>

{#if errorMsg}<p class="err">{errorMsg}</p>{/if}

<table>
  <thead>
    <tr>
      <th>Name</th>
      <th>Brand</th>
      <th>Model</th>
      <th>Variant</th>
      <th>Kind</th>
      <th class="num">Uses</th>
      <th>Status</th>
      <th>Added by</th>
      <th></th>
    </tr>
  </thead>
  <tbody>
    {#each data.items as item (item.id)}
      <tr>
        <td class="name">
          <a href="/admin/equipment/{item.id}">{item.display_name}</a>
        </td>
        <td>{item.brand || '—'}</td>
        <td>{item.model}</td>
        <td>{item.variant ?? '—'}</td>
        <td>{item.kind}</td>
        <td class="num">{item.usage_count}</td>
        <td><span class="status status--{item.status}">{item.status}</span></td>
        <td>{item.submitted_by_handle ? `@${item.submitted_by_handle}` : '—'}</td>
        <td class="actions">
          <a class="btnlink" href="/admin/equipment/{item.id}">Manage</a>
          <button
            type="button"
            class="danger"
            disabled={busy || item.usage_count > 0}
            title={item.usage_count > 0 ? 'In use — cannot delete' : 'Delete'}
            onclick={() => remove(item)}
          >
            Delete
          </button>
        </td>
      </tr>
    {:else}
      <tr><td colspan="9" class="empty">No equipment found.</td></tr>
    {/each}
  </tbody>
</table>

<div class="pager">
  <button type="button" disabled={data.page <= 0} onclick={() => gotoPage(data.page - 1)}>
    ← Prev
  </button>
  <span>Page {data.page + 1}</span>
  <button type="button" disabled={!data.has_more} onclick={() => gotoPage(data.page + 1)}>
    Next →
  </button>
</div>

<style>
  .head {
    display: flex;
    align-items: baseline;
    gap: 12px;
    margin-bottom: 16px;
  }
  h1 {
    font-family: var(--font-display, serif);
    font-weight: 400;
    margin: 0;
  }
  .count {
    color: var(--fg-muted);
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .filters {
    display: flex;
    gap: 8px;
    margin-bottom: 16px;
    flex-wrap: wrap;
  }
  .filters select,
  .filters input,
  button {
    background: var(--bg-canvas);
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 7px 10px;
    font-size: 13px;
    font-family: inherit;
  }
  .filters input {
    flex: 1;
    min-width: 180px;
  }
  button {
    cursor: pointer;
    font-family: var(--font-mono);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  button:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .btnlink {
    background: var(--bg-canvas);
    border: 1px solid var(--border-subtle);
    color: var(--fg-primary);
    padding: 7px 10px;
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    text-decoration: none;
  }
  .btnlink:hover {
    background: var(--bg-raised);
  }
  button.danger {
    border-color: var(--danger, #c33);
    color: var(--danger, #c33);
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  th,
  td {
    text-align: left;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border-subtle);
    vertical-align: middle;
  }
  th {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--fg-muted);
  }
  td.name {
    font-weight: 500;
  }
  td.name a {
    color: var(--fg-primary);
    text-decoration: none;
  }
  td.name a:hover {
    color: var(--accent);
    text-decoration: underline;
  }
  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .actions {
    display: flex;
    gap: 6px;
    white-space: nowrap;
  }
  .status {
    font-family: var(--font-mono);
    font-size: 10px;
    text-transform: uppercase;
  }
  .status--approved {
    color: var(--fg-muted);
  }
  .status--pending {
    color: var(--accent);
  }
  .status--merged,
  .status--rejected {
    color: var(--danger, #c33);
  }
  .empty {
    text-align: center;
    color: var(--fg-muted);
    padding: 32px;
  }
  .err {
    color: var(--danger, #c33);
    font-family: var(--font-mono);
    font-size: 12px;
    margin: 0 0 12px;
  }
  .pager {
    display: flex;
    align-items: center;
    gap: 16px;
    justify-content: center;
    margin-top: 20px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--fg-muted);
  }
</style>
