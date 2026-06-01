// Per-kind spec field definitions for the equipment catalog setup builder.
// Used to render the right spec inputs inside SpecsPanel for each equipment kind.
//
// IMPORTANT: this table must list EVERY editable column of each `<kind>_specs`
// row (except DB-computed fields, marked `type: 'computed'`). The specs save is
// replace-all — any populated column NOT rendered here is dropped to NULL on
// save. Keep it in sync with the spec structs in backend `api_types.rs`.

export type SpecField =
  | {
      name: string;
      label: string;
      type: 'enum';
      options: { value: string; label: string }[];
      helpText?: string;
    }
  | {
      name: string;
      label: string;
      type: 'number';
      min?: number;
      max?: number;
      step?: number;
      unit?: string;
      helpText?: string;
    }
  | { name: string; label: string; type: 'text'; helpText?: string }
  | { name: string; label: string; type: 'bool'; helpText?: string }
  | { name: string; label: string; type: 'computed'; helpText?: string };

export const TELESCOPE_FIELDS: SpecField[] = [
  {
    name: 'design',
    label: 'Design',
    type: 'enum',
    options: [
      { value: 'refractor_apo', label: 'Refractor APO' },
      { value: 'refractor_achro', label: 'Refractor achro' },
      { value: 'sct', label: 'SCT' },
      { value: 'rc', label: 'RC' },
      { value: 'newtonian', label: 'Newtonian' },
      { value: 'maksutov_cassegrain', label: 'Maksutov-Cassegrain' },
      { value: 'maksutov_newtonian', label: 'Maksutov-Newtonian' },
      { value: 'dall_kirkham', label: 'Dall-Kirkham' },
      { value: 'other', label: 'Other' }
    ]
  },
  {
    name: 'aperture_mm',
    label: 'Aperture (mm)',
    type: 'number',
    min: 30,
    max: 1500,
    step: 1,
    unit: 'mm'
  },
  {
    name: 'focal_length_mm',
    label: 'Focal length (mm)',
    type: 'number',
    min: 100,
    max: 15000,
    step: 1,
    unit: 'mm'
  },
  {
    name: 'focal_ratio_f',
    label: 'Focal ratio · computed',
    type: 'computed',
    helpText: 'focal_length_mm / aperture_mm — DB-generated, read-only.'
  },
  {
    name: 'self_weight_kg',
    label: 'Self weight (kg)',
    type: 'number',
    min: 0,
    step: 0.01,
    unit: 'kg'
  },
  {
    name: 'optical_length_mm',
    label: 'Optical length (mm)',
    type: 'number',
    min: 0,
    step: 1,
    unit: 'mm'
  },
  { name: 'backfocus_mm', label: 'Backfocus (mm)', type: 'number', min: 0, step: 0.1, unit: 'mm' }
];

export const CAMERA_FIELDS: SpecField[] = [
  {
    name: 'sensor_type',
    label: 'Sensor type',
    type: 'enum',
    options: [
      { value: 'cmos', label: 'CMOS' },
      { value: 'ccd', label: 'CCD' }
    ]
  },
  {
    name: 'color_type',
    label: 'Color type',
    type: 'enum',
    options: [
      { value: 'mono', label: 'Mono' },
      { value: 'osc', label: 'OSC (one-shot color)' }
    ]
  },
  { name: 'cooled', label: 'Cooled', type: 'bool' },
  { name: 'sensor_model', label: 'Sensor model', type: 'text', helpText: 'e.g. IMX571, KAF-8300' },
  {
    name: 'pixel_size_um',
    label: 'Pixel size (µm)',
    type: 'number',
    min: 0.5,
    max: 25,
    step: 0.01,
    unit: 'µm'
  },
  { name: 'sensor_width_px', label: 'Sensor width (px)', type: 'number', min: 1, step: 1 },
  { name: 'sensor_height_px', label: 'Sensor height (px)', type: 'number', min: 1, step: 1 },
  { name: 'self_weight_g', label: 'Self weight (g)', type: 'number', min: 0, step: 1, unit: 'g' },
  {
    name: 'full_well_capacity_e',
    label: 'Full-well capacity (e⁻)',
    type: 'number',
    min: 0,
    step: 1
  },
  { name: 'read_noise_e', label: 'Read noise (e⁻)', type: 'number', min: 0, step: 0.1 },
  { name: 'mount_thread', label: 'Mount thread', type: 'text', helpText: 'e.g. M42, M48, T2' },
  { name: 'backfocus_mm', label: 'Backfocus (mm)', type: 'number', min: 0, step: 0.1, unit: 'mm' }
];

export const FILTER_FIELDS: SpecField[] = [
  {
    name: 'filter_type',
    label: 'Type',
    type: 'enum',
    options: [
      { value: 'luminance', label: 'Luminance (L)' },
      { value: 'red', label: 'Red (R)' },
      { value: 'green', label: 'Green (G)' },
      { value: 'blue', label: 'Blue (B)' },
      { value: 'h_alpha', label: 'Hα (656 nm)' },
      { value: 'oiii', label: 'OIII (501 nm)' },
      { value: 'sii', label: 'SII (672 nm)' },
      { value: 'uv_ir_cut', label: 'UV/IR cut' },
      { value: 'dual_band', label: 'Dual-band (Hα+OIII)' },
      { value: 'tri_band', label: 'Tri-band (SHO)' },
      { value: 'quad_band', label: 'Quad-band' },
      { value: 'light_pollution', label: 'Light pollution' },
      { value: 'broadband_color', label: 'Broadband colour set' },
      { value: 'other', label: 'Other' }
    ]
  },
  {
    name: 'bandwidth_nm',
    label: 'Bandwidth (nm)',
    type: 'number',
    min: 0.1,
    max: 200,
    step: 0.1,
    unit: 'nm',
    helpText: 'Only for R/G/B/Hα/OIII/SII and multi-band filters.'
  },
  {
    name: 'size',
    label: 'Size',
    type: 'enum',
    options: [
      { value: '1_25in', label: '1.25 inch' },
      { value: '2in', label: '2 inch' },
      { value: '31mm', label: '31 mm unmounted' },
      { value: '36mm', label: '36 mm unmounted' },
      { value: '50mm_round', label: '50 mm round' },
      { value: '50mm_square', label: '50 mm square' },
      { value: 'other', label: 'Other' }
    ]
  },
  { name: 'mounted', label: 'Threaded cell', type: 'bool' },
  {
    name: 'mounted_diameter_mm',
    label: 'Mounted diameter (mm)',
    type: 'number',
    min: 0,
    step: 0.1,
    unit: 'mm'
  },
  { name: 'thickness_mm', label: 'Thickness (mm)', type: 'number', min: 0, step: 0.1, unit: 'mm' },
  {
    name: 'peak_transmission_pct',
    label: 'Peak transmission (%)',
    type: 'number',
    min: 0,
    max: 100,
    step: 0.1,
    unit: '%'
  }
];

export const MOUNT_FIELDS: SpecField[] = [
  {
    name: 'mount_type',
    label: 'Type',
    type: 'enum',
    options: [
      { value: 'equatorial_german', label: 'Equatorial German' },
      { value: 'equatorial_fork', label: 'Equatorial fork' },
      { value: 'alt_az', label: 'Alt-Az' },
      { value: 'harmonic_drive', label: 'Harmonic drive' },
      { value: 'strain_wave', label: 'Strain wave' },
      { value: 'other', label: 'Other' }
    ]
  },
  {
    name: 'payload_kg',
    label: 'Payload (kg)',
    type: 'number',
    min: 0.5,
    max: 200,
    step: 0.1,
    unit: 'kg'
  },
  { name: 'goto', label: 'GoTo', type: 'bool' },
  {
    name: 'self_weight_kg',
    label: 'Self weight (kg)',
    type: 'number',
    min: 0,
    step: 0.01,
    unit: 'kg'
  },
  {
    name: 'periodic_error_arcsec',
    label: 'Periodic error (arcsec)',
    type: 'number',
    min: 0,
    step: 0.1,
    unit: '″'
  },
  { name: 'tripod_included', label: 'Tripod included', type: 'bool' },
  {
    name: 'control_protocol',
    label: 'Control protocol',
    type: 'text',
    helpText: 'e.g. EQMOD, INDI, ASCOM'
  }
];

export const FOCAL_MODIFIER_FIELDS: SpecField[] = [
  {
    name: 'modifier_type',
    label: 'Type',
    type: 'enum',
    options: [
      { value: 'reducer', label: 'Reducer' },
      { value: 'flattener', label: 'Flattener' },
      { value: 'reducer_flattener', label: 'Reducer + flattener' },
      { value: 'barlow', label: 'Barlow' },
      { value: 'extender', label: 'Extender' },
      { value: 'corrector', label: 'Corrector' }
    ]
  },
  {
    name: 'factor',
    label: 'Factor (×)',
    type: 'number',
    min: 0.1,
    max: 5,
    step: 0.01,
    helpText: '1.0 = pure flattener · 0.79 = typical reducer · 2.0 = Barlow ×2'
  },
  { name: 'self_weight_g', label: 'Self weight (g)', type: 'number', min: 0, step: 1, unit: 'g' },
  { name: 'backfocus_mm', label: 'Backfocus (mm)', type: 'number', min: 0, step: 0.1, unit: 'mm' },
  {
    name: 'image_circle_mm',
    label: 'Image circle (mm)',
    type: 'number',
    min: 0,
    step: 0.1,
    unit: 'mm'
  }
];

export const GUIDING_FIELDS: SpecField[] = [
  {
    name: 'setup_kind',
    label: 'Setup',
    type: 'enum',
    helpText: 'Required for guiding specs.',
    options: [
      { value: 'oag', label: 'Off-axis guider (OAG)' },
      { value: 'guidescope', label: 'Guidescope' },
      { value: 'oag_prism', label: 'OAG prism' },
      { value: 'other', label: 'Other' }
    ]
  },
  {
    name: 'guide_focal_mm',
    label: 'Guide focal length (mm)',
    type: 'number',
    min: 0,
    step: 1,
    unit: 'mm'
  },
  {
    name: 'guide_aperture_mm',
    label: 'Guide aperture (mm)',
    type: 'number',
    min: 0,
    step: 1,
    unit: 'mm'
  },
  { name: 'guide_camera', label: 'Guide camera', type: 'text', helpText: 'e.g. ZWO ASI120MM Mini' }
];

export const FIELDS_BY_KIND = {
  telescope: TELESCOPE_FIELDS,
  camera: CAMERA_FIELDS,
  filter: FILTER_FIELDS,
  mount: MOUNT_FIELDS,
  focal_modifier: FOCAL_MODIFIER_FIELDS,
  guiding: GUIDING_FIELDS
} as const;
