# Equipment Handoff Conformance — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring the four equipment-catalog surfaces (verify, equip browse, setup builder, shared components) into conformance with the `docs/superpowers/handoff/2026-05-14-equipment-catalog/` design handoff. Fix the 14 conformance gaps identified in the 2026-05-16 UI/UX audit.

**Architecture:** Pure frontend work, no backend changes. The handoff is authoritative for visuals; the 2026-05-14 spec for data/API. Four independent batches (components → verify → equip browse → settings). Components fixes happen first because they are dependencies for verify. Each batch ends with a commit and `just check` green.

**Tech Stack:** SvelteKit + Svelte 5 runes, design tokens in `frontend/src/app.css`, no new dependencies.

**User decisions (clarifying questions, 2026-05-16):**
- Stepper 3→4 = relabel only (no flow restructure).
- `Follow` button = out of scope (no backend).
- Right rail: hide transmission curve, keep siblings.
- Notes on verify: keep `TagInput`, document as intentional deviation.

---

## Batch 1 — Shared components fixes

### Task 1.1: Fix `FilterChipInput` dropdown rows to show type + bandwidth

**Files:**
- Modify: `frontend/src/lib/components/equipment/FilterChipInput.svelte:~289-301`

**Context:** Handoff `chips.jsx:144-154` renders each dropdown row with the filter type label (e.g. `"Hα · 3 NM"`). Current impl always shows `"UNTYPED"`. The component already imports `FILTER_TYPE_META` and has a `bandwidthLabel()` helper in `filter-types.ts`.

- [ ] **Step 1: Locate the dropdown row meta render**

Open `FilterChipInput.svelte` and find the dropdown row rendering block (around line 289-301). It looks like:

```svelte
<span class="meta">UNTYPED</span>
```

- [ ] **Step 2: Replace with type + bandwidth derived from the filter item**

The item rendered in a dropdown row has shape `PhotoFilterChip` (id, display_name, filter_type, bandwidth_nm). Compute the meta inline:

```svelte
{#each matches as item (item.id)}
  {@const meta = item.filter_type ? FILTER_TYPE_META[item.filter_type] : null}
  {@const bw = bandwidthLabel(item.filter_type, item.bandwidth_nm)}
  <button class="dropdown-row" ...>
    <span class="name">{item.display_name}</span>
    <span class="meta">
      {#if meta}
        {meta.label.toUpperCase()}{#if bw} · {bw}{/if}
      {:else}
        UNTYPED
      {/if}
    </span>
  </button>
{/each}
```

Keep existing imports (`FILTER_TYPE_META`, `bandwidthLabel`) — they're already in `filter-types.ts`. Verify both are imported at the top of the file; add to the import line if missing.

- [ ] **Step 3: Type-check**

Run from `frontend/`:
```
pnpm check
```
Expected: 0 errors.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/lib/components/equipment/FilterChipInput.svelte
git commit -m "fix(equip): show filter type + bandwidth in chip-input dropdown rows"
```

---

### Task 1.2: Fix `FilterChipInput` dropdown counter to show "N OF M"

**Files:**
- Modify: `frontend/src/lib/components/equipment/FilterChipInput.svelte:~270`

**Context:** Handoff `chips.jsx:135` renders `"{matches.length} OF {available}"`. Current impl shows `"{matches.length} RESULTS"`.

- [ ] **Step 1: Locate the counter render and substitute**

Replace:
```svelte
<div class="meta-row">{matches.length} RESULTS</div>
```
with:
```svelte
{@const available = all.length - selectedIds.size}
<div class="meta-row">{matches.length} OF {available}</div>
```

Note: `all` is the full catalog array and `selectedIds` is the Set of already-picked chips. Verify both exist in scope (they should — they're used to compute `matches`). If `available` would be a duplicate name, choose `remaining`.

- [ ] **Step 2: Type-check**

```
cd frontend && pnpm check
```
Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/lib/components/equipment/FilterChipInput.svelte
git commit -m "fix(equip): chip-input counter shows N OF M remaining"
```

---

### Task 1.3: Fix `Field` label "AUTO-FILL" → "YOU FILL"

**Files:**
- Modify: `frontend/src/lib/components/equipment/Field.svelte:~26`

**Context:** Handoff `shell.jsx:11` writes `"YOU FILL"` for user-filled fields; current impl writes `"AUTO-FILL"` which is a different concept (auto-fill ≈ browser autocomplete).

- [ ] **Step 1: Replace the label**

In `Field.svelte`, find:
```svelte
<span class="detected detected-auto">○ AUTO-FILL</span>
```
Replace with:
```svelte
<span class="detected detected-auto">○ YOU FILL</span>
```

- [ ] **Step 2: Type-check**

```
cd frontend && pnpm check
```
Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/lib/components/equipment/Field.svelte
git commit -m "fix(equip): rename Field user-fill badge to YOU FILL per handoff"
```

---

## Batch 2 — Verify screen (`/upload/[id]/verify`)

### Task 2.1: Relabel `UploadStepper` from 3 to 4 steps

**Files:**
- Modify: `frontend/src/lib/components/UploadStepper.svelte:~16`
- Modify: `frontend/src/routes/upload/[id]/verify/+page.svelte:144` (currentStep value)

**Context:** Handoff stepper has 4 labels: `UPLOAD · VERIFY DATA · EQUIPMENT · CAPTION & PUBLISH`. Current has 3: `UPLOAD · VERIFY EACH · PUBLISH`. Per user decision, this is a relabel only (same monopage form).

- [ ] **Step 1: Read the current stepper to understand props**

```
Read frontend/src/lib/components/UploadStepper.svelte
```
Note the current `steps` constant and `currentStep` prop.

- [ ] **Step 2: Update the steps array**

In `UploadStepper.svelte`, replace the 3-step list with:
```ts
const STEPS = [
  { id: 1, label: 'UPLOAD' },
  { id: 2, label: 'VERIFY DATA' },
  { id: 3, label: 'EQUIPMENT' },
  { id: 4, label: 'CAPTION & PUBLISH' },
];
```

The verify page should pass `currentStep={2}` — `VERIFY DATA` and `EQUIPMENT` are both rendered on this page but step 2 is the "active" one (data is the primary intent). Keep currentStep=2 in `+page.svelte:144`.

- [ ] **Step 3: Check stepper is used elsewhere**

Run:
```bash
grep -rn "UploadStepper" frontend/src/routes/
```

Update `currentStep` values in other pages if they reference step 3 (which previously meant "PUBLISH"). Specifically:
- `/upload/[id]/caption` if it uses the stepper, pass `currentStep={4}`.

- [ ] **Step 4: Type-check + commit**

```
cd frontend && pnpm check
git add frontend/src/lib/components/UploadStepper.svelte frontend/src/routes/upload
git commit -m "feat(upload): 4-step stepper per equipment-catalog handoff"
```

---

### Task 2.2: Pass `startOpen={true}` to FilterChipInput on verify

**Files:**
- Modify: `frontend/src/routes/upload/[id]/verify/+page.svelte:~328-332`

**Context:** README handoff §FilterChipInput: "the dropdown should open by default in the upload-verify flow when the user first lands on the page". Component already supports `startOpen` prop.

- [ ] **Step 1: Add the prop**

Change:
```svelte
<FilterChipInput
  value={filterChips}
  orphans={data.orphans}
  onChange={(next) => (filterChips = next)}
/>
```
to:
```svelte
<FilterChipInput
  value={filterChips}
  orphans={data.orphans}
  startOpen={!isPublished}
  onChange={(next) => (filterChips = next)}
/>
```

`!isPublished` so editing an existing published photo doesn't pop the dropdown automatically.

- [ ] **Step 2: Type-check + commit**

```
cd frontend && pnpm check
git add frontend/src/routes/upload/[id]/verify/+page.svelte
git commit -m "feat(verify): chip-input dropdown opens on new-frame verify per handoff"
```

---

### Task 2.3: Add read-only EXIF table to left column of verify

**Files:**
- Modify: `frontend/src/routes/upload/[id]/verify/+page.svelte:165-194` (the `.preview` aside)
- Modify: `frontend/src/routes/upload/[id]/verify/+page.svelte` styles (add `.exif` table CSS)

**Context:** Handoff `screen-verify.jsx:67-79` renders a `<table class="exif">` under the photo preview with rows for Camera / Sensor / Sub exposure / Gain·Offset / Sensor temp / Frames captured. This is read-only display; the editable form on the right keeps the inputs.

Source values (joined fields from `data.photo`):
- **Camera** = `data.photo.camera ?? '—'`
- **Sensor temp** = `data.photo.sensor_temp_c != null ? "{n} °C" : '—'`
- **Sub exposure** = `data.photo.exposure_s != null ? "{n} s" : '—'`
- **Gain · Offset** = `data.photo.gain` and `data.photo.offset` (check schema); render `"{gain} · {offset}"` or '—'
- **ISO** = `data.photo.iso ?? '—'` (in place of "Sensor model" since we don't carry that on photo)
- **Light frames** = `data.photo.sessions ?? '—'` (label "Frames captured")

- [ ] **Step 1: Check the photo schema for `offset` field**

```bash
grep -n "offset" frontend/src/lib/api/types.ts | head -5
```

If `offset` is not on `Photo`, drop the Gain · Offset row (use only `gain`).

- [ ] **Step 2: Insert EXIF table after preview-meta**

In `+page.svelte`, inside `<aside class="preview">`, after the `<div class="preview-meta">...</div>` block (around line 190), insert:

```svelte
<table class="exif">
  <tbody>
    <tr><th>Camera</th><td class="mono">{data.photo.camera ?? '—'}</td></tr>
    <tr><th>ISO</th><td class="mono">{data.photo.iso ?? '—'}</td></tr>
    <tr><th>Sub exposure</th><td class="mono">
      {data.photo.exposure_s != null ? `${data.photo.exposure_s} s` : '—'}
    </td></tr>
    <tr><th>Gain</th><td class="mono">{data.photo.gain ?? '—'}</td></tr>
    <tr><th>Sensor temp</th><td class="mono">
      {data.photo.sensor_temp_c != null ? `${data.photo.sensor_temp_c} °C` : '—'}
    </td></tr>
    <tr><th>Frames captured</th><td class="mono">{data.photo.sessions ?? '—'}</td></tr>
  </tbody>
</table>
```

- [ ] **Step 3: Add `.exif` table styles**

In the `<style>` block at the bottom of the file (find existing `.preview-meta` selector and add below it):

```css
.exif {
  margin-top: 20px;
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
.exif th {
  text-align: left;
  font-family: var(--font-mono);
  font-weight: 400;
  color: var(--fg-muted);
  text-transform: uppercase;
  letter-spacing: 0.08em;
  font-size: 11px;
  padding: 6px 16px 6px 0;
  border-bottom: 1px solid var(--border-subtle);
  vertical-align: top;
  white-space: nowrap;
}
.exif td {
  padding: 6px 0;
  border-bottom: 1px solid var(--border-subtle);
  color: var(--fg-secondary);
}
.exif td.mono {
  font-family: var(--font-mono);
}
.exif tr:last-child th,
.exif tr:last-child td {
  border-bottom: none;
}
```

- [ ] **Step 4: Type-check + commit**

```
cd frontend && pnpm check
git add frontend/src/routes/upload/[id]/verify/+page.svelte
git commit -m "feat(verify): add read-only EXIF table under photo preview per handoff"
```

---

### Task 2.4: Group acquisition fields under labeled section + adjust left column to 520px

**Files:**
- Modify: `frontend/src/routes/upload/[id]/verify/+page.svelte:~395-400` (layout grid template)
- Modify: `frontend/src/routes/upload/[id]/verify/+page.svelte:~219-286` (insert ACQUISITION section header before the EXIF input grid; alternatively split the grid)

**Context:** Handoff `screen-verify.jsx` separates the form into labeled sections: EXIF data fields are not under "ACQUISITION" in the design — the ACQUISITION section is specifically the 4 fields: Light frames / Sub exposure / Total integration / Stacking. The current impl mixes everything in one 2-col grid.

For minimal restructure, label the existing EXIF grid as "ACQUISITION & FRAMING" (since it carries both acquisition-relevant fields like exposure_s/gain/sessions and framing-relevant fields like focal_mm/aperture_f/ra/dec). This honors the section-header intent without over-decomposing.

- [ ] **Step 1: Change left column width 560→520px**

Find:
```css
.layout {
  ...
  grid-template-columns: 560px 1fr;
  ...
}
```
Change `560px` to `520px`.

- [ ] **Step 2: Add ACQUISITION section header**

Just before the `<div class="grid">` block that wraps lens/iso/exposure_s/etc. (around line 220), insert:

```svelte
<div class="t-label section-label">ACQUISITION & FRAMING</div>
```

And add the style at the bottom of the `<style>` block:
```css
.section-label {
  margin: 24px 0 12px;
  color: var(--fg-muted);
}
```

- [ ] **Step 3: Add EQUIPMENT section header before the equipment grid**

Around line 309 (just before `<div class="grid equipment-grid">`), insert:

```svelte
<div class="t-label section-label">EQUIPMENT</div>
```

- [ ] **Step 4: Type-check + commit**

```
cd frontend && pnpm check
git add frontend/src/routes/upload/[id]/verify/+page.svelte
git commit -m "feat(verify): label form sections + tighten left column to 520px"
```

---

## Batch 3 — Equip browse (`/equip/[kind]/[slug]`)

### Task 3.1: Bump `DiscoveryHeader` equipment h1 to 64px + adjust eyebrow

**Files:**
- Modify: `frontend/src/lib/components/discovery/DiscoveryHeader.svelte:~118-133` (equipment variant block)
- Modify: `frontend/src/lib/components/discovery/DiscoveryHeader.svelte:~202-205` (`.display-equipment` CSS)

**Context:** Handoff requires h1 at 64px (display serif), and an eyebrow that reads `"<KIND> · <SUB-CATEGORY> · N ITEMS IN CATALOG"` rather than the current breadcrumb-like `"EQUIPMENT · FILTER · /EQUIP/FILTER/<slug>"`. We don't have a "sub-category" axis in the DB; render `"<KIND> · N ITEMS IN CATALOG"` instead.

- [ ] **Step 1: Update eyebrow text**

In the `{:else if props.variant === 'equipment'}` block, replace the `<p class="eyebrow">…</p>` line with:

```svelte
<p class="eyebrow">
  ● {EQUIPMENT_KIND_LABELS[meta.kind] ?? meta.kind.toUpperCase()} · {fmt(meta.photo_count)} {Number(meta.photo_count) === 1 ? 'PHOTO' : 'PHOTOS'} IN CATALOG
</p>
```

(We use photo count as the catalog quantity. If `meta` exposes a different field for "items in catalog", prefer it; the current `EquipmentMeta` ts-rs type does not. Verify via `grep "photo_count" frontend/src/lib/api/types.ts` if needed.)

- [ ] **Step 2: Resize h1 to 64px**

In the styles, change:
```css
.display-equipment {
  font-size: 48px;
  font-style: italic;
}
```
to:
```css
.display-equipment {
  font-size: 64px;
  font-style: italic;
  line-height: 1.05;
}
```

- [ ] **Step 3: Type-check + visually verify on dev server**

```
cd frontend && pnpm check
```

Verify with chrome-devtools-mcp on `/equip/filter/<any-slug>` that h1 reads 64px and eyebrow shows new text.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/lib/components/discovery/DiscoveryHeader.svelte
git commit -m "feat(equip): equipment-detail h1 to 64px + reworded eyebrow"
```

---

### Task 3.2: Add header action buttons (Edit specs, Add to setup)

**Files:**
- Modify: `frontend/src/lib/components/discovery/DiscoveryHeader.svelte:~127-132` (equipment variant `<div class="header-right">`)

**Context:** Handoff has 3 buttons (Follow / Edit specs / Add to setup). Per user, `Follow` is out of scope. Keep `Edit specs` (links to existing `/equip/[kind]/[slug]/edit/`) + `Add to setup` (links to `/settings/equipment/new?prefill=<itemId>` — the route's load handles the prefill best-effort; if not yet, link to `/settings/equipment/new` plain).

- [ ] **Step 1: Check what `/settings/equipment/new` accepts**

```bash
grep -n "prefill\|searchParams\|url\." frontend/src/routes/settings/equipment/new/+page.server.ts frontend/src/routes/settings/equipment/new/+page.svelte
```

If prefill is supported, use the query param; otherwise link plain.

- [ ] **Step 2: Add the buttons block above the stat in header-right**

Locate (around line 127):
```svelte
<div class="header-right" style="display: flex; gap: 32px; align-items: flex-end;">
  <div class="stat">…</div>
</div>
```

Change to:
```svelte
<div class="header-right">
  <div class="header-actions">
    <a class="btn btn-ghost" href="/equip/{meta.kind}/{meta.slug}/edit">Edit specs</a>
    <a class="btn btn-primary" href="/settings/equipment/new">Add to setup</a>
  </div>
  <div class="stat">
    <div class="stat-n">{fmt(meta.photo_count)}</div>
    <div class="stat-l">FRAMES</div>
  </div>
</div>
```

Add the inline-style removal: replace the `style="display: flex; gap: 32px; align-items: flex-end;"` with a class — we want a column layout (actions on top, stat below). Add to the style block:

```css
.header-right {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 16px;
}
.header-actions {
  display: flex;
  gap: 8px;
}
```

Note: the `.btn .btn-ghost .btn-primary` global classes already exist in `app.css` (verify with `grep "btn-primary" frontend/src/app.css`); reuse them.

- [ ] **Step 3: Type-check + visually verify**

```
cd frontend && pnpm check
```
Verify the buttons render aligned right of the h1, with primary amber for "Add to setup".

- [ ] **Step 4: Commit**

```bash
git add frontend/src/lib/components/discovery/DiscoveryHeader.svelte
git commit -m "feat(equip): header buttons Edit specs + Add to setup per handoff"
```

---

### Task 3.3: Add right-rail item meta card on equip browse

**Files:**
- Create: `frontend/src/lib/components/equipment/EquipmentMetaCard.svelte`
- Modify: `frontend/src/routes/equip/[kind]/[slug]/+page.svelte:~232` (place card next to siblings rail)
- Modify: `frontend/src/routes/equip/[kind]/[slug]/+page.server.ts` (ensure status/canonical/created/approved/submitted_by are loaded)

**Context:** Handoff right rail shows a card with: STATUS (colored chip), CANONICAL NAME, CREATED, APPROVED, SUBMITTED BY. The existing `data.item` from `GET /api/equipment/items/:id` already exposes these (see `backend/src/equipment/items_get.rs`). If `data.item` is null (no specs), still show the meta card.

- [ ] **Step 1: Verify `data.item` exposes the required fields**

```bash
grep -n "status\|canonical_name\|approved_at\|created_at\|submitted_by" backend/src/equipment/items_get.rs
```

If any are missing from the response struct, they need to be added — but per the 2026-05-14 spec, migration 0018 added all of these to `equipment_items`. Verify the Rust handler exposes them.

If not exposed, **stop and add** to the response struct and run `just types` (and `cargo sqlx prepare`). Document the addition in the commit.

- [ ] **Step 2: Create EquipmentMetaCard.svelte**

```svelte
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
```

- [ ] **Step 3: Render it in the equip browse page**

In `frontend/src/routes/equip/[kind]/[slug]/+page.svelte` (around line 232, near `<EquipmentPairedRail … />`), import and render:

```svelte
import EquipmentMetaCard from '$lib/components/equipment/EquipmentMetaCard.svelte';
…
{#if data.item}
  <EquipmentMetaCard item={data.item} />
{/if}
<EquipmentPairedRail items={data.initial.paired} />
```

- [ ] **Step 4: Type-check + commit**

```
cd frontend && pnpm check
git add frontend/src/lib/components/equipment/EquipmentMetaCard.svelte frontend/src/routes/equip/[kind]/[slug]/+page.svelte
git commit -m "feat(equip): catalog item meta card on browse page per handoff"
```

---

### Task 3.4: Rename `EquipmentPairedRail` label to match handoff siblings semantics

**Files:**
- Modify: `frontend/src/lib/components/discovery/EquipmentPairedRail.svelte:~11` (label text)

**Context:** Handoff says siblings = "Other Antlia narrowband" (same brand / category siblings), not "Often paired with" (co-used items). The semantics differ but the *placement* matches. Since changing the backend siblings query is out of scope, we honor the existing data semantics and keep the label that matches it. Update the label to be more accurate:

- [ ] **Step 1: Confirm label scope**

Read the component to confirm the label exists at line 11 in current form (`<p class="rail-label">Often paired with</p>`).

- [ ] **Step 2: No edit needed — document deviation**

Add a comment to `EquipmentPairedRail.svelte` above the `<p class="rail-label">`:

```svelte
<!-- Handoff calls this "Other <brand>" siblings (same canonical brand prefix).
     Current backend returns co-used items instead. Label kept honest. -->
<p class="rail-label">Often paired with</p>
```

- [ ] **Step 3: Commit**

```bash
git add frontend/src/lib/components/discovery/EquipmentPairedRail.svelte
git commit -m "docs(equip): note paired-rail semantics differ from handoff siblings"
```

---

## Batch 4 — Settings/equipment (Telescope DB-GENERATED callout)

### Task 4.1: Add DB-GENERATED callout under `focal_ratio_f` in Telescope SpecsPanel

**Files:**
- Modify: `frontend/src/routes/settings/equipment/[id]/edit/+page.svelte` (the Telescope spec rendering) **and** `frontend/src/routes/settings/equipment/new/+page.svelte`
- Or, if SetupForm is shared: modify `frontend/src/lib/components/equipment/SetupForm.svelte:~320` (after the computed `focal_ratio` Field render)

**Context:** Handoff `screen-setup.jsx:148-152` puts a `<Callout tone="info" label="DB-GENERATED">` callout spanning 2 grid columns after the computed `focal_ratio_f` field, explaining it's computed by Postgres. There is no existing `Callout.svelte` in `frontend/src/lib/components/`. Build an inline minimal callout — no need to extract a component for one usage.

- [ ] **Step 1: Find the Telescope spec render in SetupForm**

```bash
grep -n "focal_ratio\|TELESCOPE_FIELDS" frontend/src/lib/components/equipment/SetupForm.svelte
```

Locate the loop that renders TELESCOPE_FIELDS. The `focal_ratio_f` Field is rendered as part of that loop (type='computed').

- [ ] **Step 2: Insert callout after the loop for kind=telescope**

Just after the `{#each TELESCOPE_FIELDS as field}…{/each}` block, when the panel is rendering telescope specs, add:

```svelte
{#if kind === 'telescope'}
  <div class="callout-db" style="grid-column: span 2;">
    <span class="t-label">DB-GENERATED</span>
    <span class="callout-body">
      <code>focal_ratio_f</code> is a STORED column ·
      <code>focal_length_mm / aperture_mm</code> · not user-editable.
    </span>
  </div>
{/if}
```

(If the rendering structure does not naturally provide `kind`, gate on the field list variable — e.g. only telescope panels include `focal_ratio_f` of type='computed', so condition on that.)

- [ ] **Step 3: Add `.callout-db` styles to the same component**

```css
.callout-db {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 12px 16px;
  background: var(--bg-accent-tint, rgba(232, 164, 58, 0.07));
  border-left: 2px solid var(--accent);
  font-size: 12px;
  color: var(--fg-secondary);
  line-height: 1.5;
}
.callout-db .t-label {
  color: var(--accent);
}
.callout-db code {
  font-family: var(--font-mono);
  color: var(--fg-primary);
}
```

If `--bg-accent-tint` is not defined in `app.css`, the `rgba(...)` fallback applies. Verify with `grep "bg-accent-tint" frontend/src/app.css`.

- [ ] **Step 4: Type-check + visually verify**

```
cd frontend && pnpm check
```
Open `/settings/equipment/new` (or `[id]/edit`), expand the Telescope role's SpecsPanel; the callout should appear under the 4 fields.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/lib/components/equipment/SetupForm.svelte
git commit -m "feat(equip): DB-GENERATED callout under focal_ratio_f in Telescope panel"
```

---

## Final verification

### Task F.1: End-of-branch quality gate

- [ ] **Step 1: Run full check**

From repo root:
```
just check
```
Expected: green. If clippy/svelte-check fails, fix and re-run.

- [ ] **Step 2: Visual smoke via chrome-devtools-mcp**

Start dev (`just dev`), then via mcp__chrome-devtools tools, navigate to and screenshot each surface:
- `/equip/filter/<any>` — confirm h1 64px, buttons, meta card, siblings rail.
- `/upload/<photo-id>/verify` — confirm 4-step stepper, EXIF table on left, section labels, chip-input dropdown open, type/bandwidth in dropdown rows.
- `/settings/equipment/new` — expand Telescope role, confirm DB-GENERATED callout.

- [ ] **Step 3: Manual diff vs handoff index.html**

Open `docs/superpowers/handoff/2026-05-14-equipment-catalog/index.html` in a browser. Pan/zoom to each artboard, compare to the corresponding live page side-by-side. Note any remaining deltas in a tracking comment on the PR.

- [ ] **Step 4: Update audit document with status**

If the audit doc exists at a known location, mark each gap as resolved with the relevant commit hash. Otherwise, skip.

---

## Self-review notes

- All 14 audit gaps mapped to a task: ✓
- Each task contains exact paths + code: ✓
- No backend migrations or schema changes: ✓ (Task 3.3 may require exposing fields in the existing handler — flagged in-step)
- No new dependencies: ✓
- TDD not applied (UI-only changes, the project convention is e2e via chrome-devtools-mcp at end-of-branch per user feedback): noted

**Out of scope (documented):**
- `Follow` button (no backend).
- Transmission curve SVG (per user, dropped from MVP).
- True 4-page verify flow (per user, stepper is relabel only).
- Notes textarea regression (per user, keep TagInput).
- "Other &lt;brand&gt;" sibling semantics on right rail (backend returns co-used items; label updated to honest text only).
- Tabs (Photos / Used with / Discussion / History) on equip browse — not addressed; tracked for follow-up.
