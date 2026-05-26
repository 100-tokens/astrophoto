import type { FilterIntegration } from '$lib/api/FilterIntegration';

/** Per-filter integration time = subs × per-sub exposure (seconds). */
export function rowTotalS(r: FilterIntegration): number {
  return (Number(r.sub_count) || 0) * (Number(r.sub_exposure_s) || 0);
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
