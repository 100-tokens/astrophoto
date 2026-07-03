/**
 * Turn a raw API error body into copy a photographer can act on.
 *
 * The upload wizard used to render `await res.text()` verbatim, so any
 * upload-init rejection showed raw JSON like
 * `{"error":"conflict","message":"conflict: file already uploaded"}`
 * in the file row. Backend error envelopes are
 * `{ "error": <code>, "message": <text> }` (see backend/src/error.rs).
 */
export function humanizeUploadError(raw: string): string {
  let code: string | undefined;
  let message: string | undefined;
  try {
    const parsed: unknown = JSON.parse(raw);
    if (parsed && typeof parsed === 'object') {
      const p = parsed as Record<string, unknown>;
      if (typeof p.error === 'string') code = p.error;
      if (typeof p.message === 'string') message = p.message;
    }
  } catch {
    /* not a JSON envelope — fall through to the raw text */
  }

  switch (code) {
    case 'conflict':
      return 'You have already uploaded this exact file — find it in your frames or drafts.';
    case 'unsupported-format':
      if (message?.includes('plate-solve')) {
        return 'XISF uploads are temporarily unavailable — try again later, or upload a JPEG/TIFF export.';
      }
      return 'This file format is not supported. Upload a JPEG, PNG, 16-bit TIFF, or XISF.';
    case 'payload-too-large':
      return 'This file is over your tier’s size limit.';
    case 'quota-exceeded':
      return 'Your storage is full — free some space in your frames, then retry.';
    case 'magic-byte-mismatch':
      return 'The file’s contents don’t match its extension — re-export it and try again.';
    case 'pending-finalize-stuck':
      return 'The file never fully arrived — retry the upload.';
    case 'rate-limited':
    case 'too-many-requests':
      return 'Too many uploads at once — wait a moment and retry.';
    case 'unauthorized':
      return 'Your session expired — sign in again.';
    default:
      break;
  }

  if (message) return message;
  const trimmed = raw.trim();
  if (!trimmed) return 'Upload failed.';
  return trimmed.length > 200 ? `${trimmed.slice(0, 200)}…` : trimmed;
}
