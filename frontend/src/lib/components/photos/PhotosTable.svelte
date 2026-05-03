<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import PhotoTitle from './PhotoTitle.svelte';

  let {
    rows
  }: {
    rows: Array<{
      id: string;
      target?: string | null;
      original_name: string;
      taken_at?: string | null;
      exposure_s?: number | null;
      is_draft: boolean;
      status: string;
      appreciation_count: number;
    }>;
  } = $props();

  let polling = $state<number | null>(null);
  let needsPolling = $derived(rows.some((r) => r.status === 'processing'));

  $effect(() => {
    if (needsPolling && polling === null) {
      polling = window.setInterval(() => invalidateAll(), 3000);
    }
    if (!needsPolling && polling !== null) {
      clearInterval(polling);
      polling = null;
    }
    return () => {
      if (polling !== null) clearInterval(polling);
    };
  });

  function formatDate(s: string | null | undefined): string {
    if (!s) return '—';
    const d = new Date(s);
    return d.toLocaleDateString('en-GB', { day: '2-digit', month: 'short', year: 'numeric' });
  }
</script>

<table class="photos-table">
  <thead>
    <tr>
      <th></th><th>Target</th><th>Captured</th><th>Integration</th>
      <th>Status</th><th>♡</th><th></th>
    </tr>
  </thead>
  <tbody>
    {#each rows as row (row.id)}
      <tr class:is-draft={row.is_draft}>
        <td class="thumb-cell">
          {#if row.status === 'ready'}
            <img src={`/api/photos/${row.id}/thumb/400`} class="thumb" alt="" />
          {:else}
            <div class="thumb placeholder">{row.status === 'failed' ? 'FAILED' : 'PROCESSING'}</div>
          {/if}
        </td>
        <td><a href="/photo/{row.id}"><PhotoTitle photo={row} size="sm" /></a></td>
        <td>{formatDate(row.taken_at)}</td>
        <td>{row.exposure_s ? `${row.exposure_s} s` : '—'}</td>
        <td>
          {#if row.status === 'processing'}<span class="chip chip-muted">PROCESSING</span>
          {:else if row.status === 'failed'}<span class="chip chip-danger">FAILED</span>
          {:else if row.is_draft}<span class="chip chip-warning">DRAFT</span>
          {:else}<span class="chip chip-accent">PUBLISHED</span>{/if}
        </td>
        <td>{row.is_draft ? '—' : row.appreciation_count}</td>
        <td>⋯</td>
      </tr>
    {/each}
  </tbody>
</table>

<style>
  .photos-table {
    width: 100%;
    border-collapse: collapse;
  }
  .photos-table th,
  .photos-table td {
    padding: 12px 8px;
    text-align: left;
    border-bottom: 1px solid var(--border-subtle);
  }
  .photos-table th {
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.12em;
    color: var(--fg-muted);
    text-transform: uppercase;
  }
  .thumb-cell {
    width: 76px;
  }
  .thumb {
    width: 60px;
    height: 60px;
    object-fit: cover;
  }
  .thumb.placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-canvas);
    font-family: var(--font-mono);
    font-size: 9px;
    letter-spacing: 0.1em;
    color: var(--fg-muted);
  }
  tr.is-draft {
    opacity: 0.78;
  }
  tr.is-draft .thumb {
    border: 1px dashed var(--warning, #c0a060);
    position: relative;
  }
  .chip {
    padding: 2px 8px;
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.08em;
  }
  .chip-accent {
    color: var(--accent);
    border: 1px solid var(--accent);
  }
  .chip-warning {
    color: var(--warning, #c0a060);
    border: 1px solid var(--warning, #c0a060);
  }
  .chip-muted {
    color: var(--fg-muted);
    border: 1px solid var(--border-default);
  }
  .chip-danger {
    color: var(--danger);
    border: 1px solid var(--danger);
  }
</style>
