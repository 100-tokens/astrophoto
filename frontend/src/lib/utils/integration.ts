import type { FilterIntegration } from '$lib/api/FilterIntegration';
import type { PhotoFilterChip } from '$lib/api/PhotoFilterChip';

/** Per-filter integration time = subs × per-sub exposure (seconds). */
export function rowTotalS(r: FilterIntegration): number {
  return (Number(r.sub_count) || 0) * (Number(r.sub_exposure_s) || 0);
}

// ── Header-alias → catalog-filter reconciliation ──────────────────────
// The master's FITS FILTER keyword is a short band alias ("R", "Ha", …).
// To link an integration row to a real catalog filter we normalise both
// sides to a canonical band token and match. filter_type is the strongest
// signal; the display_name is a fallback for chips without a typed spec.

/** Canonical band tokens we reconcile on. */
type Band = 'L' | 'R' | 'G' | 'B' | 'HA' | 'OIII' | 'SII';

const ALIAS_SYNONYMS: Record<string, Band> = {
  L: 'L',
  LUM: 'L',
  LUMINANCE: 'L',
  R: 'R',
  RED: 'R',
  G: 'G',
  GREEN: 'G',
  B: 'B',
  BLUE: 'B',
  HA: 'HA',
  HALPHA: 'HA',
  'H-ALPHA': 'HA',
  HALPHA0: 'HA',
  OIII: 'OIII',
  O3: 'OIII',
  SII: 'SII',
  S2: 'SII'
};

const FILTER_TYPE_BAND: Record<string, Band> = {
  luminance: 'L',
  red: 'R',
  green: 'G',
  blue: 'B',
  h_alpha: 'HA',
  oiii: 'OIII',
  sii: 'SII'
};

function normaliseAlias(alias: string): Band | null {
  const key = alias.trim().toUpperCase().replace(/\s+/g, '');
  return ALIAS_SYNONYMS[key] ?? null;
}

/** Derive a chip's band from its filter_type, else a trailing display token. */
function chipBand(chip: PhotoFilterChip): Band | null {
  const t = chip.filter_type ? FILTER_TYPE_BAND[String(chip.filter_type).toLowerCase()] : null;
  if (t) return t;
  // Fallback: last standalone band-ish token in the display name.
  const tokens = chip.display_name
    .toUpperCase()
    .split(/[^A-Z0-9]+/)
    .filter(Boolean);
  for (let i = tokens.length - 1; i >= 0; i--) {
    const b = ALIAS_SYNONYMS[tokens[i] as string];
    if (b) return b;
  }
  return null;
}

/**
 * Best catalog filter id for a header alias, or null when there is no
 * unambiguous match. Returns null on zero matches AND on ties (>1) so the
 * user is asked to pick rather than silently mislinked.
 */
export function matchAliasToCatalog(alias: string, chips: PhotoFilterChip[]): string | null {
  const band = normaliseAlias(alias);
  if (!band) return null;
  const matches = chips.filter((c) => chipBand(c) === band);
  return matches.length === 1 ? (matches[0]?.id ?? null) : null;
}

/** Grand total integration across all filter rows (seconds). */
export function grandTotalS(rows: FilterIntegration[]): number {
  return rows.reduce((acc, r) => acc + rowTotalS(r), 0);
}

/** Total sub-frame count across all rows. */
export function totalSubs(rows: FilterIntegration[]): number {
  return rows.reduce((acc, r) => acc + (Number(r.sub_count) || 0), 0);
}

/** Format a duration in seconds as "Hh MMm". */
export function formatHm(totalS: number): string {
  const s = Math.max(0, Math.round(totalS));
  const h = Math.floor(s / 3600);
  const m = Math.round((s - h * 3600) / 60);
  return `${h} h ${String(m).padStart(2, '0')} m`;
}
