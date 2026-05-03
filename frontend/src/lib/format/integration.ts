/**
 * Pretty-print an integration time (seconds) for the hero stats row.
 *
 *  0       → "—"
 *  45 m    → "45m"
 *  5 h     → "5h"
 *  3h 14m  → "3h 14m"
 *
 * Negative or non-finite values render as "—".
 */
export function formatIntegration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds <= 0) return '—';
  const totalMinutes = Math.floor(seconds / 60);
  const h = Math.floor(totalMinutes / 60);
  const m = totalMinutes % 60;
  if (h === 0) return `${m}m`;
  if (m === 0) return `${h}h`;
  return `${h}h ${m}m`;
}
