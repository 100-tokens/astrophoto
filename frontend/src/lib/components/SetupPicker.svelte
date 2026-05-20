<script lang="ts">
  import { untrack } from 'svelte';
  import type { SetupSummary } from '$lib/api/SetupSummary';
  import type { SetupDetail } from '$lib/api/SetupDetail';

  // Setup picker — empty-state affordance inside verify-form/EquipmentSection.
  // When a setup is already applied, the parent (EquipmentSection) renders
  // the tinted accent summary block and DOES NOT mount this picker. So the
  // picker only needs to surface the "pick one" path: an input-shaped
  // trigger styled like the rest of the form, opening a popover list of
  // saved setups. Conflict-detection on apply is unchanged from the
  // previous <select>-based implementation.

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

  let selected = $state(untrack(() => currentSetupId ?? ''));
  let busy = $state(false);
  let open = $state(false);
  let triggerEl = $state<HTMLButtonElement | null>(null);
  let popEl = $state<HTMLDivElement | null>(null);

  let selectedName = $derived(setups.find((s) => s.id === selected)?.name ?? '');

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

  async function pick(id: string) {
    open = false;
    if (!id) {
      selected = '';
      ondetach();
      return;
    }
    busy = true;
    const r = await fetch(`${API}/api/equipment/setups/${id}`, {
      credentials: 'include'
    });
    if (!r.ok) {
      busy = false;
      return;
    }
    const detail: SetupDetail = await r.json();
    busy = false;
    const projected = projectFromDetail(detail);
    const n = conflictCount(projected);

    if (n === 0) {
      selected = id;
      onapply({ setup_id: id, mode: 'fill_empty' });
      return;
    }
    const ok = confirm(`Replace ${n} field${n > 1 ? 's' : ''}?`);
    if (ok) {
      selected = id;
      onapply({ setup_id: id, mode: 'overwrite' });
    }
  }

  function toggle() {
    if (busy) return;
    open = !open;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      open = false;
      triggerEl?.focus();
    }
  }

  function onDocClick(e: MouseEvent) {
    if (!open) return;
    const t = e.target as Node;
    if (popEl?.contains(t)) return;
    if (triggerEl?.contains(t)) return;
    open = false;
  }

  $effect(() => {
    if (!open) return;
    document.addEventListener('click', onDocClick);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('click', onDocClick);
      document.removeEventListener('keydown', onKey);
    };
  });
</script>

<div class="setup-picker">
  <button
    bind:this={triggerEl}
    type="button"
    class="trigger"
    class:has-value={selected !== ''}
    aria-haspopup="listbox"
    aria-expanded={open}
    disabled={busy}
    onclick={toggle}
  >
    <span class="value">
      {#if selected}
        {selectedName}
      {:else}
        Pick a saved setup or fill manually below…
      {/if}
    </span>
    <svg class="chevron" width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
      <path
        d="M2 3.5 L5 6.5 L8 3.5"
        fill="none"
        stroke="currentColor"
        stroke-width="1.4"
        stroke-linecap="round"
      />
    </svg>
  </button>

  {#if open}
    <div bind:this={popEl} class="pop" role="listbox" tabindex="-1">
      {#if setups.length === 0}
        <div class="empty">
          No saved setups yet. Fill the equipment fields manually — you can save them as a setup
          later from your profile.
        </div>
      {:else}
        {#each setups as s (s.id)}
          <button
            type="button"
            class="opt"
            class:is-selected={s.id === selected}
            role="option"
            aria-selected={s.id === selected}
            onclick={() => pick(s.id)}
          >
            {#if s.is_default}<span class="star" aria-label="default">★</span>{/if}
            <span class="name">{s.name}</span>
          </button>
        {/each}
        {#if selected}
          <button type="button" class="opt opt-detach" onclick={() => pick('')}>
            <span class="name">— Detach current setup —</span>
          </button>
        {/if}
      {/if}
    </div>
  {/if}

  {#if busy}
    <span class="busy" aria-live="polite">loading…</span>
  {/if}
</div>

<style>
  .setup-picker {
    position: relative;
  }

  .trigger {
    width: 100%;
    height: 36px;
    padding: 0 var(--s-3);
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--bg-base);
    color: var(--fg-faint);
    border: 1px solid var(--border-default);
    border-radius: var(--r-sm);
    font-family: var(--font-ui);
    font-size: var(--t-sm);
    cursor: pointer;
    text-align: left;
    transition:
      border-color 0.15s var(--ease-out),
      box-shadow 0.15s var(--ease-out),
      color 0.15s var(--ease-out);
  }
  .trigger.has-value {
    color: var(--fg-primary);
    font-family: var(--font-mono);
    letter-spacing: 0.01em;
  }
  .trigger:hover:not(:disabled) {
    border-color: var(--border-strong);
  }
  .trigger:focus-visible {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(232, 164, 58, 0.12);
  }
  .trigger:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .trigger .chevron {
    flex: 0 0 auto;
    color: var(--fg-muted);
    transition: transform 0.15s var(--ease-out);
  }
  .trigger[aria-expanded='true'] .chevron {
    transform: rotate(180deg);
  }
  .value {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pop {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    background: var(--bg-elevated);
    border: 1px solid var(--border-default);
    border-radius: var(--r-sm);
    box-shadow: var(--shadow-lg);
    z-index: 20;
    max-height: 320px;
    overflow: auto;
    padding: 4px 0;
  }

  .empty {
    padding: 14px 16px;
    font-family: var(--font-mono);
    font-size: var(--t-xs);
    color: var(--fg-muted);
    letter-spacing: 0.02em;
    line-height: 1.5;
  }

  .opt {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--s-2);
    padding: 8px 14px;
    background: transparent;
    border: 0;
    color: var(--fg-primary);
    font-family: var(--font-mono);
    font-size: var(--t-sm);
    letter-spacing: 0.02em;
    cursor: pointer;
    text-align: left;
  }
  .opt:hover {
    background: var(--bg-raised);
  }
  .opt.is-selected {
    color: var(--accent);
  }
  .opt-detach {
    color: var(--fg-muted);
    border-top: 1px solid var(--border-subtle);
    margin-top: 4px;
    padding-top: 12px;
  }
  .opt-detach:hover {
    color: var(--fg-secondary);
    background: var(--bg-raised);
  }
  .star {
    color: var(--accent);
    font-size: 10px;
  }
  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .busy {
    position: absolute;
    right: 32px;
    top: 50%;
    transform: translateY(-50%);
    font-family: var(--font-mono);
    font-size: var(--t-xs);
    color: var(--fg-muted);
    pointer-events: none;
  }
</style>
