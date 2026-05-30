/**
 * Whole days until the next new moon, from the mean synodic cycle.
 *
 * This is the *mean* lunation (no perturbation terms), accurate to ~1 day —
 * good enough for the "plan a dark-sky session" eyebrow on /explore, not for
 * ephemeris work. Reference new moon: 2000-01-06 18:14 UTC.
 */
const SYNODIC_DAYS = 29.530588853;
const REF_NEW_MOON_MS = Date.UTC(2000, 0, 6, 18, 14, 0);

export function daysToNextNewMoon(now: Date = new Date()): number {
  const elapsed = (now.getTime() - REF_NEW_MOON_MS) / 86_400_000;
  const intoCycle = ((elapsed % SYNODIC_DAYS) + SYNODIC_DAYS) % SYNODIC_DAYS;
  return Math.round(SYNODIC_DAYS - intoCycle);
}
