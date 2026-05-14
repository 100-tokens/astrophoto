// Per-kind spec field definitions for the equipment catalog setup builder.
// Used to render the right spec inputs inside SpecsPanel for each equipment kind.

export type SpecField =
	| { name: string; label: string; type: 'enum'; options: { value: string; label: string }[]; helpText?: string }
	| { name: string; label: string; type: 'number'; min?: number; max?: number; step?: number; unit?: string; helpText?: string }
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
			{ value: 'other', label: 'Other' },
		],
	},
	{ name: 'aperture_mm', label: 'Aperture (mm)', type: 'number', min: 30, max: 1500, step: 1, unit: 'mm' },
	{ name: 'focal_length_mm', label: 'Focal length (mm)', type: 'number', min: 100, max: 15000, step: 1, unit: 'mm' },
	{
		name: 'focal_ratio_f',
		label: 'Focal ratio · computed',
		type: 'computed',
		helpText: 'focal_length_mm / aperture_mm — DB-generated, read-only.',
	},
];

export const CAMERA_FIELDS: SpecField[] = [
	{
		name: 'sensor_type',
		label: 'Sensor type',
		type: 'enum',
		options: [
			{ value: 'cmos', label: 'CMOS' },
			{ value: 'ccd', label: 'CCD' },
		],
	},
	{
		name: 'color_type',
		label: 'Color type',
		type: 'enum',
		options: [
			{ value: 'mono', label: 'Mono' },
			{ value: 'osc', label: 'OSC (one-shot color)' },
		],
	},
	{ name: 'cooled', label: 'Cooled', type: 'bool' },
	{ name: 'sensor_model', label: 'Sensor model', type: 'text', helpText: 'e.g. IMX571, KAF-8300' },
	{ name: 'pixel_size_um', label: 'Pixel size (µm)', type: 'number', min: 0.5, max: 25, step: 0.01, unit: 'µm' },
	{ name: 'sensor_width_px', label: 'Sensor width (px)', type: 'number', min: 1, step: 1 },
	{ name: 'sensor_height_px', label: 'Sensor height (px)', type: 'number', min: 1, step: 1 },
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
			{ value: 'other', label: 'Other' },
		],
	},
	{
		name: 'bandwidth_nm',
		label: 'Bandwidth (nm)',
		type: 'number',
		min: 0.1,
		max: 200,
		step: 0.1,
		unit: 'nm',
		helpText: 'Only for R/G/B/Hα/OIII/SII and multi-band filters.',
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
			{ value: 'other', label: 'Other' },
		],
	},
	{ name: 'mounted', label: 'Threaded cell', type: 'bool' },
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
			{ value: 'other', label: 'Other' },
		],
	},
	{ name: 'payload_kg', label: 'Payload (kg)', type: 'number', min: 0.5, max: 200, step: 0.1, unit: 'kg' },
	{ name: 'goto', label: 'GoTo', type: 'bool' },
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
			{ value: 'corrector', label: 'Corrector' },
		],
	},
	{
		name: 'factor',
		label: 'Factor (×)',
		type: 'number',
		min: 0.1,
		max: 5,
		step: 0.01,
		helpText: '1.0 = pure flattener · 0.79 = typical reducer · 2.0 = Barlow ×2',
	},
];

export const FIELDS_BY_KIND = {
	telescope: TELESCOPE_FIELDS,
	camera: CAMERA_FIELDS,
	filter: FILTER_FIELDS,
	mount: MOUNT_FIELDS,
	focal_modifier: FOCAL_MODIFIER_FIELDS,
} as const;
