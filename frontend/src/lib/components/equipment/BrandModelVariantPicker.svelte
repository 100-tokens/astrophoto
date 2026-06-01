<script lang="ts">
  // Cascading, duplicate-avoiding Brand → Model → Variant picker. Each field
  // is an assisted ComboBox whose list-of-values comes from the existing
  // catalog for the kind: brands for the kind, models for the chosen brand,
  // variants for the chosen brand+model. Free entry is always allowed (new
  // gear is enterable); the goal is to *steer* toward existing values so the
  // catalog doesn't sprout "SkyWatcher" next to "Sky-Watcher".

  import ComboBox from './ComboBox.svelte';
  import type { CatalogValue } from '$lib/api/CatalogValue';
  import type { CatalogValues } from '$lib/api/CatalogValues';

  export type BMV = { brand: string; model: string; variant: string };

  type Props = {
    kind: string;
    value: BMV;
    disabled?: boolean;
    onChange: (next: BMV) => void;
  };

  let { kind, value, disabled = false, onChange }: Props = $props();

  let brands = $state<CatalogValue[]>([]);
  let models = $state<CatalogValue[]>([]);
  let variants = $state<CatalogValue[]>([]);

  async function loadValuesFor(brand: string, model: string) {
    const params = new URLSearchParams({ kind });
    if (brand.trim()) params.set('brand', brand.trim());
    if (model.trim()) params.set('model', model.trim());
    try {
      const r = await fetch(`/api/equipment/catalog-values?${params}`, { credentials: 'include' });
      if (!r.ok) return;
      const v = (await r.json()) as CatalogValues;
      brands = v.brands;
      models = v.models;
      variants = v.variants;
    } catch {
      // Non-fatal: the inputs still work as plain free text without the LOV.
    }
  }

  // Debounced cascade: refetch the LOVs ~200ms after brand/model settle, so
  // typing doesn't fire a request per keystroke while the lists stay in sync
  // (pick/clear a brand → its models load; pick a model → its variants load).
  $effect(() => {
    const k = kind;
    const b = value.brand;
    const m = value.model;
    void k;
    const t = setTimeout(() => void loadValuesFor(b, m), 200);
    return () => clearTimeout(t);
  });
</script>

<div class="bmv">
  <ComboBox
    label="Brand"
    required
    options={brands}
    value={value.brand}
    {disabled}
    placeholder="e.g. Sky-Watcher"
    onChange={(brand) => onChange({ ...value, brand })}
  />
  <ComboBox
    label="Model"
    required
    options={models}
    value={value.model}
    {disabled}
    placeholder="e.g. Esprit 100 ED"
    onChange={(model) => onChange({ ...value, model })}
  />
  <ComboBox
    label="Variant"
    options={variants}
    value={value.variant}
    {disabled}
    placeholder={'optional — e.g. "Pro", "Mk II"'}
    onChange={(variant) => onChange({ ...value, variant })}
  />
</div>

<style>
  .bmv {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
  }
  @media (max-width: 720px) {
    .bmv {
      grid-template-columns: 1fr;
    }
  }
</style>
