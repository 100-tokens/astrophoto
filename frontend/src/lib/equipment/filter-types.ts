// Per-filter-type metadata: badge code, label, CSS accent colour, and
// whether to render `bandwidth_nm` next to the chip name. Mirrors the
// table in docs/superpowers/specs/2026-05-14-equipment-catalog-enriched-design.md
// §"FilterChip — type → badge code / color mapping" and the handoff's
// `chips.css`.

import type { FilterType } from '$lib/api/FilterType';

export type FilterTypeMeta = {
	code: string; // badge text ("L", "R", "Hα", "OIII", "?", ...)
	label: string; // human label
	cssVar: string; // value for --ft-c (CSS variable or hex)
	showBandwidth: boolean; // whether to render bandwidth_nm
};

export const FILTER_TYPE_META: Record<FilterType, FilterTypeMeta> = {
	luminance: {
		code: 'L',
		label: 'Luminance',
		cssVar: 'var(--fg-primary)',
		showBandwidth: false,
	},
	red: { code: 'R', label: 'Red', cssVar: '#c25048', showBandwidth: true },
	green: { code: 'G', label: 'Green', cssVar: '#7da64a', showBandwidth: true },
	blue: { code: 'B', label: 'Blue', cssVar: '#6b8db8', showBandwidth: true },
	h_alpha: {
		code: 'Hα',
		label: 'Hydrogen alpha',
		cssVar: '#b04634',
		showBandwidth: true,
	},
	oiii: {
		code: 'OIII',
		label: 'Oxygen III',
		cssVar: '#4ea0a8',
		showBandwidth: true,
	},
	sii: {
		code: 'SII',
		label: 'Sulphur II',
		cssVar: 'var(--accent)',
		showBandwidth: true,
	},
	uv_ir_cut: {
		code: 'UV/IR',
		label: 'UV/IR cut',
		cssVar: '#8a6a9c',
		showBandwidth: false,
	},
	dual_band: {
		code: 'D',
		label: 'Dual band',
		cssVar: '#7a8fa8',
		showBandwidth: true,
	},
	tri_band: {
		code: 'T',
		label: 'Tri band',
		cssVar: '#7a9588',
		showBandwidth: true,
	},
	quad_band: {
		code: 'Q',
		label: 'Quad band',
		cssVar: '#8a8a6a',
		showBandwidth: true,
	},
	light_pollution: {
		code: 'LP',
		label: 'Light pollution',
		cssVar: 'var(--warning)',
		showBandwidth: false,
	},
	broadband_color: {
		code: 'BB',
		label: 'Broadband colour',
		cssVar: 'var(--fg-secondary)',
		showBandwidth: false,
	},
	other: {
		code: '?',
		label: 'Other',
		cssVar: 'var(--fg-faint)',
		showBandwidth: false,
	},
};

export function bandwidthLabel(filter: {
	filter_type?: FilterType | null;
	bandwidth_nm?: number | null;
}): string | null {
	if (filter.bandwidth_nm == null) return null;
	if (!filter.filter_type) return null;
	if (!FILTER_TYPE_META[filter.filter_type].showBandwidth) return null;
	return `${filter.bandwidth_nm} nm`;
}
