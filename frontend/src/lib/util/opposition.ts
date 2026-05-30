// Formats a target's opposition / midnight-culmination day-of-year for display.
//
// `opposition_doy` (from the API) is the day-of-year a fixed object sits
// opposite the Sun and transits at local midnight — its best-observation date.
// It is approximate (±a few days, and the strict "opposition" vs clock-midnight
// conventions differ by the equation of time), so we deliberately render it at
// month granularity ("early/mid/late <Mon>") rather than an exact date.

const MONTHS = [
  'Jan',
  'Feb',
  'Mar',
  'Apr',
  'May',
  'Jun',
  'Jul',
  'Aug',
  'Sep',
  'Oct',
  'Nov',
  'Dec'
] as const;

/**
 * "early Oct", "mid Jun", "late Dec" — or "" when the date is unknown.
 * Uses the same non-leap reference calendar as the backend (doy 1 = Jan 1),
 * so day-of-year ↔ month/day is stable and matches the stored value.
 */
export function formatOpposition(doy: number | null | undefined): string {
  if (doy == null) return '';
  const clamped = Math.max(1, Math.min(365, Math.round(doy)));
  const date = new Date(Date.UTC(2025, 0, clamped));
  const dom = date.getUTCDate();
  const band = dom <= 10 ? 'early' : dom <= 20 ? 'mid' : 'late';
  return `${band} ${MONTHS[date.getUTCMonth()]}`;
}
