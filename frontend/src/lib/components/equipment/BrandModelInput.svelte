<script lang="ts">
  // Shared Brand + Model (+ optional Variant) input row for catalog v2
  // saisie-forcée create flows. Sits at the top of every per-kind
  // *SpecsForm under FilterChipInput / EquipmentAutocomplete / SetupForm.
  //
  // The Brand input uses a native <datalist> seeded from KNOWN_BRANDS so
  // common brands are one keystroke away, while free entry stays allowed
  // (the value is plain user text, not constrained to the list).
  //
  // The component is purely controlled — it never POSTs. Parents own
  // the submit cycle and read `value` / `value.brand` etc. on their
  // own confirm action.

  import { onMount } from 'svelte';
  import { KNOWN_BRANDS } from '$lib/equipment/brands';
  import type { CatalogValues } from '$lib/api/CatalogValues';

  export type BrandModel = {
    brand: string;
    model: string;
    variant: string;
  };

  type Props = {
    value: BrandModel;
    /** Disables the inputs (e.g. while a create POST is in flight). */
    disabled?: boolean;
    /**
     * Optional headline above the three inputs (e.g. "ZWO ASI2600MC")
     * — when present it doubles as the preview of what `display_name`
     * will look like after the backend regenerates it from the three
     * fields. Pass through `displayName` from the parent.
     */
    label?: string | null;
    /**
     * Equipment kind. When provided, the brand datalist is seeded from the
     * EXISTING catalog brands for that kind (most-used first) so users pick a
     * real value instead of minting "SkyWatcher" next to "Sky-Watcher".
     * Falls back to the static KNOWN_BRANDS list when absent or on error.
     */
    kind?: string;
    onChange: (next: BrandModel) => void;
  };

  let { value, disabled = false, label = null, kind = undefined, onChange }: Props = $props();

  // Catalog-derived brand suggestions for the kind (loaded once on mount).
  let catalogBrands = $state<string[]>([]);
  onMount(async () => {
    if (!kind) return;
    try {
      const r = await fetch(`/api/equipment/catalog-values?kind=${encodeURIComponent(kind)}`, {
        credentials: 'include'
      });
      if (r.ok) catalogBrands = ((await r.json()) as CatalogValues).brands.map((b) => b.value);
    } catch {
      // Non-fatal — the static KNOWN_BRANDS list still backs the datalist.
    }
  });

  // Existing catalog brands first, then any KNOWN_BRANDS not already present.
  let brandOptions = $derived.by(() => {
    const seen = new Set(catalogBrands.map((b) => b.toLowerCase()));
    return [...catalogBrands, ...KNOWN_BRANDS.filter((b) => !seen.has(b.toLowerCase()))];
  });

  // Stable id for the brand datalist — avoid collisions when two
  // BrandModelInputs co-exist on the same page (rare but possible).
  const datalistId = `bm-brands-${Math.random().toString(36).slice(2, 8)}`;

  function update(patch: Partial<BrandModel>) {
    onChange({ ...value, ...patch });
  }
</script>

<div class="bm-input">
  {#if label}
    <div class="bm-label">{label}</div>
  {/if}

  <div class="ec-create-row is-split">
    <span class="ec-create-label is-required">Brand · Model</span>
    <input
      class="ec-create-input"
      list={datalistId}
      placeholder="e.g. ZWO"
      autocomplete="off"
      spellcheck="false"
      value={value.brand}
      {disabled}
      aria-label="Brand"
      oninput={(e) => update({ brand: (e.target as HTMLInputElement).value })}
    />
    <input
      class="ec-create-input"
      placeholder="e.g. ASI2600MC"
      autocomplete="off"
      spellcheck="false"
      value={value.model}
      {disabled}
      aria-label="Model"
      oninput={(e) => update({ model: (e.target as HTMLInputElement).value })}
    />
  </div>

  <div class="ec-create-row">
    <span class="ec-create-label">
      Variant
      <span class="ec-create-hint">optional</span>
    </span>
    <input
      class="ec-create-input"
      placeholder={'e.g. "Pro", "Mk II"'}
      autocomplete="off"
      spellcheck="false"
      value={value.variant}
      {disabled}
      aria-label="Variant"
      oninput={(e) => update({ variant: (e.target as HTMLInputElement).value })}
    />
  </div>

  <datalist id={datalistId}>
    {#each brandOptions as brand (brand)}
      <option value={brand}></option>
    {/each}
  </datalist>
</div>

<style>
  .bm-input {
    display: flex;
    flex-direction: column;
    gap: var(--s-2);
  }

  .bm-label {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
    text-transform: uppercase;
    letter-spacing: 0.12em;
  }
</style>
