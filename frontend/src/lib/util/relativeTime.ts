/**
 * Compact, mono-friendly "time ago" for discovery tiles — e.g. "3 H", "2 D",
 * "5 MIN", "NOW". Returns '' for missing/future timestamps so callers can
 * conditionally render. Caller appends "AGO" (except for "NOW").
 */
export function timeAgoShort(iso: string): string {
  const diff = Date.now() - Date.parse(iso);
  if (!Number.isFinite(diff) || diff < 0) return '';
  const mins = Math.floor(diff / 60_000);
  if (mins < 1) return 'NOW';
  if (mins < 60) return `${mins} MIN`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours} H`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days} D`;
  const months = Math.floor(days / 30);
  if (months < 12) return `${months} MO`;
  return `${Math.floor(days / 365)} Y`;
}
