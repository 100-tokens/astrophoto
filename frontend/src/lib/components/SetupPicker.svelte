<script lang="ts">
  import { untrack } from 'svelte';
  import type { SetupSummary } from '$lib/api/SetupSummary';
  import type { SetupDetail } from '$lib/api/SetupDetail';

  interface Current {
    scope: string;
    focal_modifier: string;
    camera: string;
    mount: string;
    filters: string;
    guiding: string;
  }

  interface Props {
    setups: SetupSummary[];
    currentSetupId: string | null;
    current: Current;
    onapply: (req: { setup_id: string; mode: 'fill_empty' | 'overwrite' }) => void;
    ondetach: () => void;
  }

  let { setups, currentSetupId, current, onapply, ondetach }: Props = $props();

  const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '';

  // Initialized once from the parent's prop. Reset imperatively on apply/detach/cancel.
  // External prop changes never needed here: the only writers to photo_setup_id are
  // this picker's own onapply/ondetach callbacks. `untrack` makes the
  // one-shot read explicit so svelte-check stops flagging it as a missed
  // reactivity dependency.
  let selected = $state(untrack(() => currentSetupId ?? ''));
  let busy = $state(false);

  function projectFromDetail(d: SetupDetail): Current {
    let scope = '';
    let focal_modifier = '';
    let camera = '';
    let mount = '';
    const filterNames: string[] = [];
    for (const it of d.items) {
      switch (it.role) {
        case 'optical_tube':
          scope = it.item.display_name;
          break;
        case 'focal_modifier':
          focal_modifier = it.item.display_name;
          break;
        case 'main_camera':
          camera = it.item.display_name;
          break;
        case 'mount':
          mount = it.item.display_name;
          break;
        case 'filter':
          filterNames.push(it.item.display_name);
          break;
      }
    }
    filterNames.sort((a, b) => a.localeCompare(b));
    return {
      scope,
      focal_modifier,
      camera,
      mount,
      filters: filterNames.join(', '),
      guiding: d.guiding ?? ''
    };
  }

  function conflictCount(target: Current): number {
    const fields: (keyof Current)[] = [
      'scope',
      'focal_modifier',
      'camera',
      'mount',
      'filters',
      'guiding'
    ];
    let n = 0;
    for (const f of fields) {
      const cur = (current[f] ?? '').trim();
      const next = (target[f] ?? '').trim();
      if (cur && next && cur !== next) n++;
    }
    return n;
  }

  async function onChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    const newId = target.value;

    if (!newId) {
      // "(none)" selected — same as detach.
      selected = '';
      ondetach();
      return;
    }

    busy = true;
    const r = await fetch(`${API}/api/equipment/setups/${newId}`, {
      credentials: 'include'
    });
    if (!r.ok) {
      busy = false;
      // Revert to whatever the parent considers current.
      target.value = currentSetupId ?? '';
      selected = currentSetupId ?? '';
      return;
    }
    const detail: SetupDetail = await r.json();
    busy = false;
    const projected = projectFromDetail(detail);
    const n = conflictCount(projected);

    if (n === 0) {
      selected = newId;
      onapply({ setup_id: newId, mode: 'fill_empty' });
      return;
    }

    const ok = confirm(`Replace ${n} field${n > 1 ? 's' : ''}?`);
    if (ok) {
      selected = newId;
      onapply({ setup_id: newId, mode: 'overwrite' });
    } else {
      target.value = currentSetupId ?? '';
      selected = currentSetupId ?? '';
    }
  }

  function detach() {
    selected = '';
    ondetach();
  }
</script>

<div class="setup-picker">
  <label class="picker-label">
    <span class="t-label">Setup</span>
    <select bind:value={selected} onchange={onChange} disabled={busy}>
      <option value="">(none)</option>
      {#each setups as s (s.id)}
        <option value={s.id}>{s.is_default ? '★ ' : ''}{s.name}</option>
      {/each}
    </select>
  </label>
  {#if selected}
    <button type="button" class="btn ghost detach" onclick={detach}>Detach</button>
  {/if}
  {#if busy}
    <span class="busy" aria-live="polite">…</span>
  {/if}
</div>

<style>
  .setup-picker {
    display: flex;
    gap: 0.5rem;
    align-items: flex-end;
    flex-wrap: wrap;
  }
  .picker-label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    min-width: 18rem;
  }
  .t-label {
    font-size: 0.85em;
    color: var(--muted, #666);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  select {
    padding: 0.5rem;
    font-size: 1rem;
  }
  .btn {
    padding: 0.4rem 0.8rem;
    border-radius: 4px;
    cursor: pointer;
  }
  .btn.ghost {
    background: transparent;
    border: 1px solid var(--border, #ccc);
    color: inherit;
  }
  .busy {
    color: var(--muted, #666);
  }
</style>
