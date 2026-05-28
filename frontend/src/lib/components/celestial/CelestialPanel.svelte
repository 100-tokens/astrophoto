<script lang="ts">
  import type { CelestialObject } from '$lib/api/CelestialObject';
  import { cssVarForType } from '$lib/utils/celestial-colors';
  import CelestialObjectDetail from './CelestialObjectDetail.svelte';

  let {
    objects,
    selectedSlug = $bindable(),
    layers = $bindable(),
    showPgc = $bindable(),
    labelsAlwaysOn = $bindable(),
    isOwner,
    photoId
  }: {
    objects: CelestialObject[];
    selectedSlug: string | null;
    layers: Set<string>;
    showPgc: boolean;
    labelsAlwaysOn: boolean;
    isOwner: boolean;
    photoId: string;
  } = $props();

  // Layer pills cover the OpenNGC object_type families. PGC and "labels"
  // are separate toggles below.
  const ALL_TYPES = ['G', 'Neb', 'OCl', 'GCl', 'PN', 'HII', 'SNR', 'Cl+N'] as const;

  function togglePill(t: string) {
    const next = new Set(layers);
    if (next.has(t)) next.delete(t);
    else next.add(t);
    layers = next;
  }

  let recomputing = $state(false);
  async function recompute() {
    recomputing = true;
    try {
      const r = await fetch(`/api/photos/${photoId}/celestial-objects/recompute`, {
        method: 'POST',
        credentials: 'include'
      });
      if (r.ok) location.reload();
    } finally {
      recomputing = false;
    }
  }

  let selected = $derived(objects.find((o) => o.slug === selectedSlug) ?? null);
  let visible = $derived(
    objects
      .filter((o) => showPgc || o.kind !== 'pgc')
      .filter((o) => layers.has(o.objectType ?? 'other'))
  );
</script>

<section class="celestial-panel">
  <h5>● CELESTIAL OBJECTS · {visible.length}</h5>

  {#if !selected}
    <div class="pills" role="group" aria-label="object type layers">
      {#each ALL_TYPES as t (t)}
        <button
          type="button"
          class="pill"
          class:active={layers.has(t)}
          onclick={() => togglePill(t)}
        >{t}</button>
      {/each}
      <button
        type="button"
        class="pill"
        class:active={showPgc}
        onclick={() => (showPgc = !showPgc)}
      >PGC</button>
      <button
        type="button"
        class="pill"
        class:active={labelsAlwaysOn}
        onclick={() => (labelsAlwaysOn = !labelsAlwaysOn)}
      >labels</button>
    </div>

    <ul class="list">
      {#each visible as o (o.slug)}
        <li>
          <button
            type="button"
            class="row"
            onclick={() => (selectedSlug = o.slug)}
          >
            <span class="dot" style="border-color: var({cssVarForType(o.objectType)})"></span>
            <span class="name">{o.canonicalName}</span>
            <span class="type">{o.objectType ?? ''}</span>
          </button>
        </li>
      {/each}
    </ul>

    {#if isOwner}
      <button
        type="button"
        class="recompute"
        onclick={recompute}
        disabled={recomputing}
      >{recomputing ? 'recomputing…' : '↻ recompute'}</button>
    {/if}
  {:else}
    <CelestialObjectDetail object={selected} onBack={() => (selectedSlug = null)} />
  {/if}
</section>

<style>
  .celestial-panel {
    border: 1px solid var(--accent);
    padding: 8px 10px;
    border-radius: var(--r-md);
    background: var(--bg-accent-tint);
    margin: 8px 0;
  }
  h5 {
    margin: 0 0 6px;
    font-size: 11px;
    color: var(--accent);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .pills {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
    margin-bottom: 8px;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .pill {
    background: var(--bg-raised);
    border: 0;
    color: var(--fg-secondary);
    padding: 2px 8px;
    border-radius: var(--r-pill);
    font-size: 11px;
    cursor: pointer;
    font-family: ui-monospace, monospace;
  }
  .pill.active {
    background: var(--accent);
    color: var(--accent-ink);
  }
  .list {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  .list .row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 2px;
    width: 100%;
    background: none;
    border: 0;
    cursor: pointer;
    font-size: 12px;
    color: var(--fg-primary);
    text-align: left;
    font-family: inherit;
  }
  .list .row:hover {
    background: rgba(255, 255, 255, 0.04);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    border-width: 1px;
    border-style: solid;
    flex-shrink: 0;
  }
  .type {
    margin-left: auto;
    color: var(--fg-muted);
    font-size: 10px;
  }
  .recompute {
    margin-top: 8px;
    background: none;
    border: 0;
    color: var(--fg-muted);
    font-size: 11px;
    cursor: pointer;
    font-family: ui-monospace, monospace;
  }
  .recompute:disabled {
    cursor: not-allowed;
    opacity: 0.6;
  }
</style>
