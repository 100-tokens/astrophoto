<script lang="ts">
  import { page } from '$app/state';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import AppFooter from '$lib/components/AppFooter.svelte';
  import Img from '$lib/components/Img.svelte';
  import type { EquipmentSpecsPayload } from '$lib/api/EquipmentSpecsPayload';
  import type { PageData } from './$types';

  let { data }: { data: PageData } = $props();

  let item = $derived(data.item);
  let specs = $derived(item.specs);

  // ── Display copy / SEO ──────────────────────────────────────────
  const KIND_LABELS: Record<string, string> = {
    telescope: 'Telescope',
    camera: 'Camera',
    mount: 'Mount',
    filter: 'Filter',
    focal_modifier: 'Focal modifier',
    guiding: 'Guiding'
  };

  let kindLabel = $derived(KIND_LABELS[item.kind] ?? item.kind);
  let pageTitle = $derived(
    `${item.brand || ''} ${item.model}${item.variant ? ` ${item.variant}` : ''} — ${kindLabel} — Astrophoto`
      .trim()
      .replace(/\s+/g, ' ')
  );
  let pageDescription = $derived(
    `Spec sheet and photos for ${item.brand || ''} ${item.model} on Astrophoto.`.trim()
  );
  let canonical = $derived(
    `${page.url.origin}/equip/${encodeURIComponent(item.kind)}/${encodeURIComponent(item.canonical_name.replace(/\s+/g, '-'))}`
  );

  // ── Photos using this item ─────────────────────────────────────
  // Photos pre-resolved by the server load (discovery handler under
  // the hood — see `+page.server.ts` for the rationale and the known
  // filter-multi-row limitation).
  let photoTiles = $derived(data.photos);

  // ── Spec table — per-kind rows, split into required / optional ──
  interface SpecRow {
    label: string;
    value: string;
    required: boolean;
  }

  const TELESCOPE_DESIGN_LABELS: Record<string, string> = {
    refractor_apo: 'Refractor APO',
    refractor_achro: 'Refractor Achro',
    sct: 'SCT',
    rc: 'RC',
    newtonian: 'Newtonian',
    maksutov_cassegrain: 'Maksutov-Cassegrain',
    maksutov_newtonian: 'Maksutov-Newtonian',
    dall_kirkham: 'Dall-Kirkham',
    other: 'Other'
  };
  const MOUNT_TYPE_LABELS: Record<string, string> = {
    equatorial_german: 'Equatorial — German',
    equatorial_fork: 'Equatorial — Fork',
    alt_az: 'Alt-Az',
    harmonic_drive: 'Harmonic drive',
    strain_wave: 'Strain wave',
    other: 'Other'
  };
  const FILTER_TYPE_LABELS: Record<string, string> = {
    luminance: 'Luminance',
    red: 'Red',
    green: 'Green',
    blue: 'Blue',
    h_alpha: 'H-alpha',
    oiii: 'OIII',
    sii: 'SII',
    uv_ir_cut: 'UV/IR cut',
    dual_band: 'Dual band',
    tri_band: 'Tri band',
    quad_band: 'Quad band',
    light_pollution: 'Light pollution',
    broadband_color: 'Broadband color',
    other: 'Other'
  };
  const FOCAL_MODIFIER_LABELS: Record<string, string> = {
    reducer: 'Reducer',
    flattener: 'Flattener',
    reducer_flattener: 'Reducer/Flattener',
    barlow: 'Barlow',
    extender: 'Extender',
    corrector: 'Corrector'
  };
  const GUIDING_KIND_LABELS: Record<string, string> = {
    oag: 'OAG',
    guidescope: 'Guidescope',
    oag_prism: 'OAG prism',
    other: 'Other'
  };
  const FILTER_SIZE_LABELS: Record<string, string> = {
    '1_25in': '1.25"',
    '2in': '2"',
    '31mm': '31 mm',
    '36mm': '36 mm',
    '50mm_round': '50 mm round',
    '50mm_square': '50 mm square',
    other: 'Other'
  };

  function fmtVal(v: unknown, suffix = ''): string {
    if (v === null || v === undefined || v === '') return '—';
    return `${v}${suffix}`;
  }

  function buildRows(p: EquipmentSpecsPayload | null): SpecRow[] {
    if (!p) return [];
    switch (p.kind) {
      case 'telescope':
        return [
          {
            label: 'Design',
            value: p.design ? (TELESCOPE_DESIGN_LABELS[p.design] ?? p.design) : '—',
            required: true
          },
          { label: 'Aperture', value: fmtVal(p.aperture_mm, ' mm'), required: true },
          { label: 'Focal length', value: fmtVal(p.focal_length_mm, ' mm'), required: true },
          {
            label: 'Focal ratio',
            value: p.focal_ratio_f != null ? `f/${p.focal_ratio_f.toFixed(2)}` : '—',
            required: false
          },
          { label: 'Self weight', value: fmtVal(p.self_weight_kg, ' kg'), required: false },
          { label: 'Optical length', value: fmtVal(p.optical_length_mm, ' mm'), required: false },
          { label: 'Backfocus', value: fmtVal(p.backfocus_mm, ' mm'), required: false }
        ];
      case 'camera':
        return [
          {
            label: 'Sensor type',
            value: p.sensor_type ? p.sensor_type.toUpperCase() : '—',
            required: true
          },
          {
            label: 'Color',
            value: p.color_type === 'mono' ? 'Mono' : p.color_type === 'osc' ? 'OSC' : '—',
            required: true
          },
          {
            label: 'Cooled',
            value: p.cooled === true ? 'Yes' : p.cooled === false ? 'No' : '—',
            required: true
          },
          { label: 'Sensor model', value: fmtVal(p.sensor_model), required: false },
          { label: 'Pixel size', value: fmtVal(p.pixel_size_um, ' µm'), required: false },
          {
            label: 'Resolution',
            value:
              p.sensor_width_px != null && p.sensor_height_px != null
                ? `${p.sensor_width_px} × ${p.sensor_height_px} px`
                : '—',
            required: false
          },
          { label: 'Self weight', value: fmtVal(p.self_weight_g, ' g'), required: false },
          {
            label: 'Full-well capacity',
            value: fmtVal(p.full_well_capacity_e, ' e⁻'),
            required: false
          },
          { label: 'Read noise', value: fmtVal(p.read_noise_e, ' e⁻'), required: false },
          { label: 'Mount thread', value: fmtVal(p.mount_thread), required: false },
          { label: 'Backfocus', value: fmtVal(p.backfocus_mm, ' mm'), required: false }
        ];
      case 'mount':
        return [
          {
            label: 'Type',
            value: p.mount_type ? (MOUNT_TYPE_LABELS[p.mount_type] ?? p.mount_type) : '—',
            required: true
          },
          { label: 'Payload', value: fmtVal(p.payload_kg, ' kg'), required: true },
          {
            label: 'GoTo',
            value: p.goto === true ? 'Yes' : p.goto === false ? 'No' : '—',
            required: true
          },
          { label: 'Self weight', value: fmtVal(p.self_weight_kg, ' kg'), required: false },
          {
            label: 'Periodic error',
            value: fmtVal(p.periodic_error_arcsec, '″'),
            required: false
          },
          {
            label: 'Tripod included',
            value: p.tripod_included === true ? 'Yes' : p.tripod_included === false ? 'No' : '—',
            required: false
          },
          { label: 'Control protocol', value: fmtVal(p.control_protocol), required: false }
        ];
      case 'filter':
        return [
          {
            label: 'Type',
            value: p.filter_type ? (FILTER_TYPE_LABELS[p.filter_type] ?? p.filter_type) : '—',
            required: true
          },
          { label: 'Bandwidth', value: fmtVal(p.bandwidth_nm, ' nm'), required: true },
          {
            label: 'Size',
            value: p.size ? (FILTER_SIZE_LABELS[p.size] ?? p.size) : '—',
            required: false
          },
          {
            label: 'Mounted',
            value: p.mounted === true ? 'Yes' : p.mounted === false ? 'No' : '—',
            required: false
          },
          {
            label: 'Mounted diameter',
            value: fmtVal(p.mounted_diameter_mm, ' mm'),
            required: false
          },
          { label: 'Thickness', value: fmtVal(p.thickness_mm, ' mm'), required: false },
          {
            label: 'Peak transmission',
            value: fmtVal(p.peak_transmission_pct, '%'),
            required: false
          }
        ];
      case 'focal_modifier':
        return [
          {
            label: 'Type',
            value: p.modifier_type
              ? (FOCAL_MODIFIER_LABELS[p.modifier_type] ?? p.modifier_type)
              : '—',
            required: true
          },
          { label: 'Factor', value: p.factor != null ? `×${p.factor}` : '—', required: true },
          { label: 'Self weight', value: fmtVal(p.self_weight_g, ' g'), required: false },
          { label: 'Backfocus', value: fmtVal(p.backfocus_mm, ' mm'), required: false },
          { label: 'Image circle', value: fmtVal(p.image_circle_mm, ' mm'), required: false }
        ];
      case 'guiding':
        return [
          {
            label: 'Setup kind',
            value: p.setup_kind ? (GUIDING_KIND_LABELS[p.setup_kind] ?? p.setup_kind) : '—',
            required: true
          },
          { label: 'Guide focal', value: fmtVal(p.guide_focal_mm, ' mm'), required: false },
          { label: 'Guide aperture', value: fmtVal(p.guide_aperture_mm, ' mm'), required: false },
          { label: 'Guide camera', value: fmtVal(p.guide_camera), required: false }
        ];
      default:
        return [];
    }
  }

  let rows = $derived(buildRows(specs));
  let requiredRows = $derived(rows.filter((r) => r.required));
  let optionalRows = $derived(rows.filter((r) => !r.required));

  let joinedYear = $derived(
    item.created_at ? new Date(item.created_at).toISOString().slice(0, 10) : '—'
  );
</script>

<svelte:head>
  <title>{pageTitle}</title>
  <meta name="description" content={pageDescription} />
  <link rel="canonical" href={canonical} />
  <meta property="og:type" content="website" />
  <meta property="og:title" content={pageTitle} />
  <meta property="og:description" content={pageDescription} />
  <meta property="og:url" content={canonical} />
</svelte:head>

<AppHeader />

<main class="detail">
  <header class="hero">
    <p class="t-eyebrow accent">{kindLabel.toUpperCase()}</p>
    <h1 class="t-display">
      {#if item.brand}<span class="brand">{item.brand}</span> ·
      {/if}
      <span class="model t-display-i">{item.model}</span>
      {#if item.variant}
        <span class="variant"> · {item.variant}</span>
      {/if}
    </h1>
    <p class="stat-strip t-meta">
      <span><strong>{item.usage_count}</strong> photo{item.usage_count === 1 ? '' : 's'}</span>
      <span class="sep">·</span>
      <span
        ><strong>{Number(item.setup_count)}</strong>
        setup{Number(item.setup_count) === 1 ? '' : 's'}</span
      >
      <span class="sep">·</span>
      <span>joined the catalog {joinedYear}</span>
    </p>

    {#if data.canSeeEditAffordance}
      <div class="hero-actions">
        <a
          class="btn-edit"
          href={`/equip/${item.kind}/${item.canonical_name.replace(/\s+/g, '-')}/edit`}
        >
          Edit specs
        </a>
      </div>
    {/if}
  </header>

  <section class="specs">
    <h2 class="t-eyebrow">SPEC SHEET</h2>
    {#if specs == null}
      <p class="empty-specs t-meta">
        No specs recorded yet.
        {#if data.canSeeEditAffordance}
          <a
            class="inline-link"
            href={`/equip/${item.kind}/${item.canonical_name.replace(/\s+/g, '-')}/edit`}
            >Add the first specs →</a
          >
        {/if}
      </p>
    {:else}
      <div class="spec-block">
        <h3 class="t-eyebrow muted">REQUIRED</h3>
        <table class="spec-table">
          <tbody>
            {#each requiredRows as r (r.label)}
              <tr>
                <th>{r.label}</th>
                <td>{r.value}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      {#if optionalRows.length > 0}
        <div class="spec-block">
          <h3 class="t-eyebrow muted">OPTIONAL</h3>
          <table class="spec-table">
            <tbody>
              {#each optionalRows as r (r.label)}
                <tr>
                  <th>{r.label}</th>
                  <td>{r.value}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    {/if}
  </section>

  <section class="photos">
    <h2 class="t-eyebrow">PHOTOS USING IT</h2>
    {#if photoTiles.length === 0}
      <p class="empty-specs t-meta">No published photos use this item yet.</p>
    {:else}
      <ul class="photo-grid">
        {#each photoTiles as p (p.id)}
          <li>
            <a class="photo-tile" href={`/u/${p.author_handle}/p/${p.short_id}`}>
              <Img photoId={p.id} w={320} alt={p.target ?? 'Untitled'} class="photo-img" />
              <span class="photo-cap t-meta">{p.target ?? 'Untitled'}</span>
            </a>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <footer class="meta">
    {#if item.submitted_by_handle}
      <p class="t-meta">
        Added by <a class="handle-link" href="/u/{item.submitted_by_handle}"
          >@{item.submitted_by_handle}</a
        >
      </p>
    {/if}
    <!--
      Spec asks for a "Delete" affordance gated on usage_count==0 &&
      setup_count==0 for the original submitter. We compute the gate
      here (`canDelete`) and surface the metadata that drives it, but
      the actual delete is intentionally deferred: no backend
      `DELETE /api/equipment/items/:id` endpoint exists yet, and
      shipping a button that 404s would be worse than no button.
      Tracked for the v3 moderation pass.
    -->
  </footer>
</main>

<AppFooter />

<style>
  .detail {
    max-width: 1200px;
    margin: 0 auto;
    padding: 32px 64px 64px;
  }
  .hero {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding-bottom: 32px;
    border-bottom: 1px solid var(--border-subtle);
    position: relative;
  }
  .hero h1 {
    margin: 0;
    font-size: clamp(2.25rem, 4vw, 3.5rem);
    line-height: 1.05;
    color: var(--fg-primary);
  }
  .hero .brand,
  .hero .variant {
    color: var(--fg-secondary);
    font-style: normal;
  }
  .hero .model {
    color: var(--fg-primary);
  }
  .stat-strip {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
    color: var(--fg-muted);
    margin: 0;
  }
  .stat-strip strong {
    color: var(--fg-primary);
    font-weight: 500;
  }
  .stat-strip .sep {
    color: var(--fg-faint);
  }
  .hero-actions {
    position: absolute;
    top: 0;
    right: 0;
  }
  .btn-edit {
    display: inline-block;
    background: var(--accent);
    color: var(--accent-ink);
    font-family: var(--font-mono);
    font-size: 12px;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    padding: 8px 16px;
    border: 1px solid var(--accent);
    text-decoration: none;
    transition: background 0.12s;
  }
  .btn-edit:hover {
    background: var(--accent-hover);
  }
  .specs {
    margin-top: 40px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .specs h2 {
    margin: 0 0 4px 0;
  }
  .spec-block {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .spec-block h3 {
    margin: 0;
  }
  .spec-block h3.muted {
    color: var(--fg-faint);
  }
  .spec-table {
    width: 100%;
    border-collapse: collapse;
    background: var(--bg-raised);
    border: 1px solid var(--border-default);
  }
  .spec-table th,
  .spec-table td {
    padding: 12px 16px;
    text-align: left;
    vertical-align: top;
    border-bottom: 1px solid var(--border-subtle);
  }
  .spec-table tr:last-child th,
  .spec-table tr:last-child td {
    border-bottom: none;
  }
  .spec-table th {
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--fg-muted);
    font-weight: 500;
    width: 240px;
  }
  .spec-table td {
    color: var(--fg-primary);
    font-family: var(--font-ui);
    font-size: 14px;
  }
  .empty-specs {
    margin: 8px 0;
    color: var(--fg-muted);
  }
  .inline-link {
    color: var(--accent);
    text-decoration: none;
  }
  .inline-link:hover {
    text-decoration: underline;
  }
  .photos {
    margin-top: 48px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .photos h2 {
    margin: 0;
  }
  .photo-grid {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 12px;
  }
  .photo-tile {
    display: flex;
    flex-direction: column;
    gap: 6px;
    text-decoration: none;
    color: var(--fg-secondary);
  }
  .photo-tile :global(.photo-img) {
    width: 100%;
    aspect-ratio: 4 / 3;
    object-fit: cover;
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    transition: border-color 0.12s;
  }
  .photo-tile:hover :global(.photo-img) {
    border-color: var(--accent);
  }
  .photo-cap {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    margin-top: 48px;
    padding-top: 16px;
    border-top: 1px solid var(--border-subtle);
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
  }
  .meta p {
    margin: 0;
  }
  .handle-link {
    color: var(--accent);
    text-decoration: none;
  }
  .handle-link:hover {
    text-decoration: underline;
  }
  @media (max-width: 768px) {
    .detail {
      padding: 16px;
    }
    .hero-actions {
      position: static;
      margin-top: 12px;
    }
    .spec-table th {
      width: 140px;
      font-size: 10px;
    }
  }
</style>
