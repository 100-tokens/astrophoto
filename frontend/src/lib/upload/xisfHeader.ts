// Client-side XISF header reader for per-filter integration auto-fill.
//
// Reads ONLY the file header (first slice) — the master body (often
// hundreds of MB) is never read into memory or uploaded. Mirrors the
// fields the backend pulls in `backend/src/photos/xisf_display.rs`:
// FILTER, NCOMBINE / Process:Integration:ImageCount, and the
// PCL:TotalExposureTime F64 vector (summed across channels, as PixInsight
// displays it). Uses targeted attribute extraction rather than a DOM
// parser so it runs identically in the browser and in node-based tests.

export interface XisfHeaderFacts {
  filter: string | null;
  frames: number | null;
  totalExposureS: number | null;
  /** Per-sub exposure (s) from the FITS EXPTIME/EXPOSURE keyword, when present. */
  subExposureS: number | null;
}

const SIGNATURE = 'XISF0100';
const HEADER_SCAN_BYTES = 262_144; // 256 KB — real headers are tens of KB

export async function parseXisfHeader(file: File): Promise<XisfHeaderFacts | null> {
  const head = new Uint8Array(await file.slice(0, HEADER_SCAN_BYTES).arrayBuffer());
  if (head.length < 16) return null;
  if (String.fromCharCode(...head.subarray(0, 8)) !== SIGNATURE) return null;

  // Header length: u32 LE at byte 8.
  const headerLen = new DataView(head.buffer, head.byteOffset, 16).getUint32(8, true);
  const end = 16 + headerLen;
  const bytes = end <= head.length ? head : new Uint8Array(await file.slice(0, end).arrayBuffer());
  const xml = new TextDecoder().decode(bytes.subarray(16, Math.min(end, bytes.length)));

  const attr = (tag: string, name: string): string | null => {
    const m = new RegExp(`\\b${name}="([^"]*)"`, 'i').exec(tag);
    return m?.[1] ?? null;
  };
  const fits = (name: string): string | null => {
    for (const m of xml.matchAll(/<FITSKeyword\b[^>]*?\/?>/gi)) {
      if (attr(m[0], 'name')?.toUpperCase() === name) return attr(m[0], 'value');
    }
    return null;
  };
  // Property value may be in a `value` attribute or as element text.
  const propValue = (id: string): string | null => {
    const open = new RegExp(`<Property\\b[^>]*\\bid="${id}"[^>]*?(/?)>`, 'i').exec(xml);
    if (!open) return null;
    const fromAttr = attr(open[0], 'value');
    if (fromAttr != null) return fromAttr;
    if (open[1] === '/') return null; // self-closing, no text
    const rest = xml.slice(open.index + open[0].length);
    const close = /<\/Property>/i.exec(rest);
    const inner = close ? rest.slice(0, close.index) : '';
    return inner.replace(/<!\[CDATA\[|\]\]>/g, '').trim() || null;
  };

  const unquote = (s: string | null): string | null =>
    s == null ? null : (s.trim().replace(/^'|'$/g, '').trim() || null);
  const intOf = (s: string | null): number | null => {
    if (s == null) return null;
    const n = parseInt(s.trim(), 10);
    return Number.isFinite(n) ? n : null;
  };
  const numOf = (s: string | null): number | null => {
    if (s == null) return null;
    const n = parseFloat(unquote(s) ?? '');
    return Number.isFinite(n) && n > 0 ? n : null;
  };

  return {
    filter: unquote(fits('FILTER') ?? propValue('Instrument:Filter:Name')),
    frames: intOf(fits('NCOMBINE') ?? propValue('Process:Integration:ImageCount')),
    totalExposureS: decodeF64VecSum(propValue('PCL:TotalExposureTime')),
    // WBPP master lights carry the per-sub exposure directly as EXPTIME.
    subExposureS: numOf(fits('EXPTIME') ?? fits('EXPOSURE'))
  };
}

// base64 → little-endian f64 vector → sum (one entry per channel for a
// multi-channel integration; PixInsight displays the sum).
function decodeF64VecSum(b64: string | null): number | null {
  const raw = (b64 ?? '').trim();
  if (!raw) return null;
  let bin: Uint8Array;
  try {
    bin = Uint8Array.from(atob(raw), (c) => c.charCodeAt(0));
  } catch {
    return null;
  }
  if (bin.length === 0 || bin.length % 8 !== 0) return null;
  const dv = new DataView(bin.buffer, bin.byteOffset, bin.length);
  let sum = 0;
  for (let i = 0; i < bin.length; i += 8) sum += dv.getFloat64(i, true);
  return Number.isFinite(sum) && sum > 0 ? sum : null;
}
