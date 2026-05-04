<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import type { SetupSummary } from '$lib/api/SetupSummary';

  let { data }: { data: { setups: SetupSummary[] } } = $props();

  const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '';

  const ROLE_LABEL: Record<string, string> = {
    optical_tube: 'Telescope',
    focal_modifier: 'Focal modifier',
    main_camera: 'Camera',
    mount: 'Mount',
    filter: 'Filter'
  };

  let acting = $state<string | null>(null);
  let actError = $state<string | null>(null);

  async function setDefault(s: SetupSummary) {
    acting = s.id;
    actError = null;
    // Need full detail to PATCH (the API does replace-all on items).
    const dr = await fetch(`${API}/api/equipment/setups/${s.id}`, {
      credentials: 'include'
    });
    if (!dr.ok) {
      actError = `Could not load setup ${s.name}`;
      acting = null;
      return;
    }
    const detail = await dr.json();
    const body = {
      name: detail.name,
      description: detail.description,
      location: detail.location,
      is_remote: detail.is_remote,
      is_default: true,
      guiding: detail.guiding,
      items: detail.items.map((it: { role: string; item: { id: string } }) => ({
        role: it.role,
        item_id: it.item.id
      }))
    };
    const r = await fetch(`${API}/api/equipment/setups/${s.id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify(body)
    });
    acting = null;
    if (r.ok) {
      await invalidateAll();
    } else {
      actError = 'Could not set default';
    }
  }

  async function del(s: SetupSummary) {
    if (!confirm(`Delete setup "${s.name}"? This cannot be undone.`)) return;
    acting = s.id;
    actError = null;
    const r = await fetch(`${API}/api/equipment/setups/${s.id}`, {
      method: 'DELETE',
      credentials: 'include'
    });
    acting = null;
    if (r.status === 204) {
      await invalidateAll();
    } else {
      actError = 'Could not delete';
    }
  }
</script>

<header class="header">
  <h1>Equipment setups</h1>
  <a href="/settings/equipment/new" class="btn primary">+ New setup</a>
</header>

{#if actError}
  <p class="form-error">{actError}</p>
{/if}

{#if data.setups.length === 0}
  <p class="empty">
    No setups yet. <a href="/settings/equipment/new">Create your first one</a>.
  </p>
{:else}
  <ul class="list">
    {#each data.setups as s (s.id)}
      <li class="card" class:busy={acting === s.id}>
        <div class="head">
          <h2>{s.name}</h2>
          <div class="badges">
            {#if s.is_default}<span class="badge default">Default</span>{/if}
            {#if s.is_remote}<span class="badge">Remote</span>{/if}
          </div>
        </div>

        {#if s.description}
          <p class="desc">{s.description}</p>
        {/if}

        {#if s.item_counts.length > 0}
          <p class="counts">
            {#each s.item_counts as c (c.role)}
              <span class="count">
                {ROLE_LABEL[c.role] ?? c.role} · {c.count}
              </span>
            {/each}
          </p>
        {/if}

        <p class="updated">
          Updated {new Date(s.updated_at).toLocaleDateString()}
        </p>

        <div class="actions">
          {#if !s.is_default}
            <button
              type="button"
              class="btn ghost"
              onclick={() => setDefault(s)}
              disabled={acting === s.id}>Set as default</button
            >
          {/if}
          <a href={`/settings/equipment/${s.id}/edit`} class="btn ghost">Edit</a>
          <button type="button" class="btn danger" onclick={() => del(s)} disabled={acting === s.id}
            >Delete</button
          >
        </div>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }
  .list {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .card {
    border: 1px solid var(--border, #ccc);
    border-radius: 6px;
    padding: 1rem;
  }
  .card.busy {
    opacity: 0.5;
    pointer-events: none;
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  .badges {
    display: flex;
    gap: 0.25rem;
  }
  .badge {
    font-size: 0.75em;
    padding: 0.1rem 0.5rem;
    border-radius: 999px;
    background: var(--chip-bg, #eee);
  }
  .badge.default {
    background: var(--primary, #0a6);
    color: white;
  }
  .desc {
    color: var(--muted, #666);
    margin: 0.5rem 0;
  }
  .counts {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin: 0.5rem 0;
  }
  .count {
    background: var(--chip-bg, #eee);
    padding: 0.1rem 0.5rem;
    border-radius: 4px;
    font-size: 0.85em;
  }
  .updated {
    color: var(--muted, #666);
    font-size: 0.85em;
    margin: 0.25rem 0;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.5rem;
  }
  .btn {
    padding: 0.4rem 0.8rem;
    border-radius: 4px;
    cursor: pointer;
    text-decoration: none;
    font-size: 0.9em;
    display: inline-flex;
    align-items: center;
  }
  .btn.primary {
    background: var(--primary, #0a6);
    color: white;
    border: none;
  }
  .btn.ghost {
    background: transparent;
    border: 1px solid var(--border, #ccc);
    color: inherit;
  }
  .btn.danger {
    background: transparent;
    border: 1px solid var(--error, #c00);
    color: var(--error, #c00);
  }
  .empty {
    color: var(--muted, #666);
  }
  .form-error {
    color: var(--error, #c00);
  }
</style>
