import exifr from 'exifr';

// exifr is published as CommonJS — named imports fail under Vite SSR.
// Use the default import and pull `parse` off it.
const parseExif = exifr.parse;

export type Preflight = {
  thumbDataUrl: string;
  exif: Record<string, unknown>;
  hash: string;
  /**
   * The mime to advertise to the backend. For JPEG/PNG/TIFF this is
   * just `file.type`; for `.xisf` (which browsers don't recognise) we
   * derive it from the extension since the backend's `upload_init`
   * allowlist and S3 PUT both key off the wire mime.
   */
  mime: string;
};

/**
 * Browsers don't recognise XISF as an image type, so `<input type=file>`
 * (and File.type) returns "" for `.xisf` uploads. The backend allowlist
 * (`backend/src/photos/upload_init.rs`) and the S3 PUT both need a real
 * mime. Map `.xisf` here so the rest of the upload pipeline sees the
 * same `application/x-xisf` the backend expects.
 */
export function resolveMime(file: File): string {
  if (file.type) return file.type;
  if (/\.xisf$/i.test(file.name)) return 'application/x-xisf';
  return 'application/octet-stream';
}

export async function preflight(file: File): Promise<Preflight> {
  const mime = resolveMime(file);
  const isXisf = mime === 'application/x-xisf';
  // XISF: skip thumbnail + EXIF. The browser has no XISF decoder so
  // `createImageBitmap` throws, and XISF carries its instrumentation
  // in FITS keywords / PCL properties — not in JPEG EXIF. The display
  // image + metadata for an XISF upload land later via the backend's
  // auto-calibrate flow (see `backend/src/photos/platesolve_upload.rs`).
  const [thumbDataUrl, exif, hash] = await Promise.all([
    isXisf ? Promise.resolve('') : makeThumb(file),
    isXisf ? Promise.resolve({}) : parseExif(file).catch(() => ({})),
    sha256(file)
  ]);
  return { thumbDataUrl, exif: exif ?? {}, hash, mime };
}

async function makeThumb(file: File): Promise<string> {
  const bmp = await createImageBitmap(file, { resizeWidth: 256, resizeQuality: 'medium' });
  const canvas = document.createElement('canvas');
  canvas.width = bmp.width;
  canvas.height = bmp.height;
  canvas.getContext('2d')!.drawImage(bmp, 0, 0);
  return canvas.toDataURL('image/jpeg', 0.8);
}

async function sha256(file: File): Promise<string> {
  if (typeof crypto !== 'undefined' && crypto.subtle?.digest) {
    const buf = await file.arrayBuffer();
    const digest = await crypto.subtle.digest('SHA-256', buf);
    return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, '0')).join('');
  }
  // Insecure-context fallback: deterministic but weak. Sufficient for dedup
  // in dev when accessing via raw IP. Production runs on HTTPS so this is dead path.
  const stub = `${file.size}-${file.lastModified}-${file.name}`;
  return stub
    .split('')
    .map((c) => c.charCodeAt(0).toString(16).padStart(2, '0'))
    .join('')
    .slice(0, 64)
    .padEnd(64, '0');
}
