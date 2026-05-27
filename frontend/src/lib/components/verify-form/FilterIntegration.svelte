<script lang="ts">
  import type { FilterIntegration } from '$lib/api/FilterIntegration';
  import type { PhotoFilterChip } from '$lib/api/PhotoFilterChip';
  import { parseXisfHeader } from '$lib/upload/xisfHeader';
  import {
    rowTotalS,
    grandTotalS,
    totalSubs,
    formatHm,
    matchAliasToCatalog
  } from '$lib/utils/integration';

  interface Props {
    value: FilterIntegration[];
    /** The photo's chosen catalog filter chips — used to auto-match the
        header alias and to populate each row's "link filter" picker. */
    catalogFilters?: PhotoFilterChip[];
    disabled?: boolean;
    onChange: (next: FilterIntegration[]) => void;
  }
  let { value, catalogFilters = [], disabled = false, onChange }: Props = $props();

  const BANDS = ['L', 'R', 'G', 'B', 'Ha', 'OIII', 'SII'];
  const MAX_ROWS = 12;

  let parseNote = $state<string | null>(null);

  let total = $derived(grandTotalS(value));
  let subs = $derived(totalSubs(value));

  function update(i: number, patch: Partial<FilterIntegration>) {
    onChange(value.map((r, j) => (j === i ? { ...r, ...patch } : r)));
  }
  function add() {
    if (value.length >= MAX_ROWS) return;
    onChange([...value, { filter: '', sub_count: 0, sub_exposure_s: 0, filter_item_id: null }]);
  }
  function remove(i: number) {
    onChange(value.filter((_, j) => j !== i));
  }

  // Drop per-filter integration masters → read the header locally (never
  // uploaded) → upsert a row per filter. sub-exposure is derived as
  // total ÷ frames when both are present.
  async function onMasters(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const files = Array.from(input.files ?? []);
    input.value = '';
    let failed = 0;
    let next = [...value];
    for (const f of files) {
      const facts = await parseXisfHeader(f);
      if (
        !facts ||
        (facts.filter == null &&
          facts.frames == null &&
          facts.totalExposureS == null &&
          facts.subExposureS == null)
      ) {
        failed++;
        continue;
      }
      const label = (facts.filter ?? '').trim();
      // Prefer the direct per-sub EXPTIME; else derive total ÷ frames.
      const derived =
        facts.subExposureS ??
        (facts.frames && facts.totalExposureS
          ? Math.round((facts.totalExposureS / facts.frames) * 100) / 100
          : null);
      const idx = label
        ? next.findIndex((r) => r.filter.trim().toLowerCase() === label.toLowerCase())
        : -1;
      // Auto-match the header alias to a catalog chip (override-able). Never
      // clobber a link the user already set by hand.
      const autoLink = label ? matchAliasToCatalog(label, catalogFilters) : null;
      const cur = idx >= 0 ? next[idx] : undefined;
      if (cur) {
        next[idx] = {
          filter: cur.filter || label,
          sub_count: facts.frames ?? cur.sub_count,
          sub_exposure_s: derived ?? cur.sub_exposure_s,
          filter_item_id: cur.filter_item_id ?? autoLink ?? null
        };
      } else if (next.length < MAX_ROWS) {
        next = [
          ...next,
          {
            filter: label,
            sub_count: facts.frames ?? 0,
            sub_exposure_s: derived ?? 0,
            filter_item_id: autoLink ?? null
          }
        ];
      }
    }
    parseNote =
      failed > 0
        ? `Couldn't read ${failed} file${failed === 1 ? '' : 's'} — not an XISF master?`
        : null;
    onChange(next);
  }
</script>

<section class="fi">
  <div class="fi-head">
    <div class="t-label">PER-FILTER INTEGRATION</div>
    {#if value.length}
      <span class="t-meta">{formatHm(total)} · {subs} subs · {value.length} filters</span>
    {/if}
  </div>

  {#each value as row, i (i)}
    <div class="fi-row">
      <input
        class="input input-mono fi-filter"
        list="fi-bands"
        placeholder="L"
        value={row.filter}
        {disabled}
        oninput={(e) => update(i, { filter: e.currentTarget.value })}
        aria-label="filter"
      />
      <select
        class="input input-mono fi-link"
        class:fi-link-empty={!row.filter_item_id}
        value={row.filter_item_id ?? ''}
        disabled={disabled || catalogFilters.length === 0}
        onchange={(e) => update(i, { filter_item_id: e.currentTarget.value || null })}
        aria-label="catalog filter"
        title={catalogFilters.length === 0
          ? 'Add filters in FILTERS · STRUCTURED to link them here'
          : 'Link this band to a catalog filter'}
      >
        <option value="">— link filter —</option>
        {#each catalogFilters as c (c.id)}
          <option value={c.id}>{c.display_name}</option>
        {/each}
      </select>
      <input
        class="input input-mono"
        type="text"
        inputmode="numeric"
        placeholder="subs"
        value={row.sub_count || ''}
        {disabled}
        oninput={(e) => update(i, { sub_count: parseInt(e.currentTarget.value) || 0 })}
        aria-label="sub count"
      />
      <input
        class="input input-mono"
        type="text"
        inputmode="decimal"
        placeholder="sec"
        value={row.sub_exposure_s || ''}
        {disabled}
        oninput={(e) => update(i, { sub_exposure_s: parseFloat(e.currentTarget.value) || 0 })}
        aria-label="sub exposure seconds"
      />
      <span class="fi-total t-meta">{formatHm(rowTotalS(row))}</span>
      <button
        type="button"
        class="btn btn-ghost btn-sm"
        {disabled}
        onclick={() => remove(i)}
        aria-label="remove filter">×</button
      >
    </div>
  {/each}

  <datalist id="fi-bands">
    {#each BANDS as b}<option value={b}></option>{/each}
  </datalist>

  <div class="fi-actions">
    {#if value.length < MAX_ROWS}
      <button type="button" class="btn btn-ghost btn-sm" {disabled} onclick={add}
        >+ Add filter</button
      >
    {/if}
    <!-- No `name` attr: this file is read locally by parseXisfHeader and is
         NEVER part of the form submit / uploaded. -->
    <label class="fi-drop">
      <input type="file" accept=".xisf" multiple class="vh" {disabled} onchange={onMasters} />
      <span class="t-meta">⤓ Drop per-filter masters — header read locally, file not uploaded</span>
    </label>
  </div>
  {#if parseNote}<p class="fi-note t-meta">{parseNote}</p>{/if}
</section>

<style>
  .fi {
    margin-top: 28px;
  }
  .fi-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 12px;
  }
  .fi-row {
    display: grid;
    grid-template-columns: 0.7fr 1.4fr 0.7fr 0.7fr auto auto;
    gap: 12px;
    align-items: center;
    margin-bottom: 8px;
  }
  .fi-link {
    min-width: 0;
  }
  /* Unlinked rows read as a faint prompt, not a filled value. */
  .fi-link-empty {
    color: var(--fg-faint);
  }
  .fi-total {
    white-space: nowrap;
    min-width: 64px;
    text-align: right;
  }
  .fi-actions {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-top: 8px;
    flex-wrap: wrap;
  }
  .fi-drop {
    cursor: pointer;
    display: inline-flex;
    align-items: center;
  }
  .fi-drop:hover .t-meta {
    color: var(--accent);
  }
  .fi-note {
    color: var(--danger);
    margin: 8px 0 0;
  }
  .vh {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
  @media (max-width: 640px) {
    .fi-row {
      grid-template-columns: 1fr 1fr;
    }
    .fi-link {
      grid-column: 1 / -1;
    }
  }
</style>
