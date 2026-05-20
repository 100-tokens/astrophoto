/** Format right ascension (decimal degrees) as sexagesimal hours: "00ʰ42ᵐ44ˢ". */
export function formatRA(degrees: number): string {
  const norm = ((degrees % 360) + 360) % 360;
  const hoursTotal = norm / 15;
  const h = Math.floor(hoursTotal);
  const minutesTotal = (hoursTotal - h) * 60;
  const m = Math.floor(minutesTotal);
  const s = Math.round((minutesTotal - m) * 60);
  return `${pad(h)}ʰ${pad(m)}ᵐ${pad(s)}ˢ`;
}

/** Format declination (decimal degrees) as signed sexagesimal: "+41°16′09″". */
export function formatDec(degrees: number): string {
  const sign = degrees < 0 ? '-' : '+';
  const abs = Math.abs(degrees);
  const d = Math.floor(abs);
  const minutesTotal = (abs - d) * 60;
  const min = Math.floor(minutesTotal);
  const sec = Math.round((minutesTotal - min) * 60);
  return `${sign}${pad(d)}°${pad(min)}′${pad(sec)}″`;
}

function pad(n: number): string {
  return n.toString().padStart(2, '0');
}

// ---------------------------------------------------------------------------
// Sexagesimal-aware editing helpers used by the verify page.
//
// The verify page stores ra_deg / dec_deg as decimal-degree strings (server
// contract — see backend/src/photos/metadata.rs). The user, however, types
// in whichever notation feels natural: pure decimal degrees, HMS for RA, or
// DMS for DEC. These helpers translate between the two so the form stays a
// decimal source-of-truth while displaying the astronomer-friendly form.
//
// Heuristic for parse():
//   - contains 'h' / 'm' / 's' (case-insensitive)        → HMS  (RA only)
//   - contains '°' / '′' / '″' or ASCII ′-/″-equivalents → DMS
//   - otherwise                                          → decimal degrees
// ---------------------------------------------------------------------------

const HMS_LETTERS = /[hms]/i;
const DMS_MARKERS = /[°'′"″]/;

/** Parse a right-ascension string as either decimal degrees or HMS.
 *  Returns null on empty input or unparseable garbage. */
export function parseRaToDeg(raw: string): number | null {
  const s = raw.trim();
  if (s === '') return null;
  if (HMS_LETTERS.test(s)) {
    const parts = extractHms(s);
    if (!parts) return null;
    const [h, m, sec] = parts;
    if (h < 0 || m < 0 || sec < 0) return null;
    if (m >= 60 || sec >= 60) return null;
    return (h + m / 60 + sec / 3600) * 15;
  }
  if (DMS_MARKERS.test(s)) {
    // Permissive: some users may paste RA in DMS too. Convert as plain
    // signed degrees, no h→15° multiplier.
    return parseSignedDms(s);
  }
  return parseFiniteNumber(s);
}

/** Parse a declination string as either decimal degrees or DMS.
 *  Returns null on empty input or unparseable garbage. */
export function parseDecToDeg(raw: string): number | null {
  const s = raw.trim();
  if (s === '') return null;
  if (DMS_MARKERS.test(s)) return parseSignedDms(s);
  // DEC is never expressed in HMS; treat any h/m/s letters as invalid.
  if (HMS_LETTERS.test(s)) return null;
  return parseFiniteNumber(s);
}

/** Format decimal-degree RA as "20h 59m 17.2s" with one fractional second. */
export function formatRaHms(deg: number): string {
  if (!Number.isFinite(deg)) return '';
  const norm = ((deg % 360) + 360) % 360;
  const hoursTotal = norm / 15;
  const h = Math.floor(hoursTotal);
  const minutesTotal = (hoursTotal - h) * 60;
  const m = Math.floor(minutesTotal);
  const sec = (minutesTotal - m) * 60;
  // One decimal place keeps the round-trip stable to ~1.5″.
  return `${pad(h)}h ${pad(m)}m ${sec.toFixed(1)}s`;
}

/** Format decimal-degree DEC as "+44° 31′ 44.0″" with one fractional second. */
export function formatDecDms(deg: number): string {
  if (!Number.isFinite(deg)) return '';
  const sign = deg < 0 ? '-' : '+';
  const abs = Math.abs(deg);
  const d = Math.floor(abs);
  const minutesTotal = (abs - d) * 60;
  const min = Math.floor(minutesTotal);
  const sec = (minutesTotal - min) * 60;
  return `${sign}${pad(d)}° ${pad(min)}′ ${sec.toFixed(1)}″`;
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/** Extract three numbers from an HMS string like "20h59m17.2s" or "20h 59m 17.2s".
 *  Returns [h, m, s] or null. */
function extractHms(s: string): [number, number, number] | null {
  const m = s.match(/^\s*(-?\d+(?:\.\d+)?)\s*h\s*(\d+(?:\.\d+)?)\s*m\s*(\d+(?:\.\d+)?)\s*s\s*$/i);
  if (!m) return null;
  const h = Number(m[1]);
  const mm = Number(m[2]);
  const ss = Number(m[3]);
  if (!Number.isFinite(h) || !Number.isFinite(mm) || !Number.isFinite(ss)) return null;
  return [h, mm, ss];
}

/** Parse a signed DMS string and return decimal degrees.
 *  Accepts the unicode primes ′ / ″ or ASCII ' / ". */
function parseSignedDms(s: string): number | null {
  // Normalise unicode primes to ASCII first so the regex stays short.
  const normalised = s.replace(/′/g, "'").replace(/″/g, '"');
  const m = normalised.match(
    /^\s*([+-]?)\s*(\d+(?:\.\d+)?)\s*°\s*(?:(\d+(?:\.\d+)?)\s*'\s*)?(?:(\d+(?:\.\d+)?)\s*"\s*)?$/
  );
  if (!m) return null;
  const sign = m[1] === '-' ? -1 : 1;
  const d = Number(m[2]);
  const mm = m[3] === undefined ? 0 : Number(m[3]);
  const ss = m[4] === undefined ? 0 : Number(m[4]);
  if (!Number.isFinite(d) || !Number.isFinite(mm) || !Number.isFinite(ss)) return null;
  if (mm >= 60 || ss >= 60 || mm < 0 || ss < 0) return null;
  return sign * (d + mm / 60 + ss / 3600);
}

function parseFiniteNumber(s: string): number | null {
  // Accept the same forms as Number(), but reject empty / whitespace
  // (already filtered upstream) and non-finite results.
  const n = Number(s);
  return Number.isFinite(n) ? n : null;
}
