/**
 * Same-origin return path after sign-in.
 *
 * Callers pass `next` (settings, upload, admin) or `return` (follow,
 * appreciate, comments). Only a single-slash path on this origin is
 * honored. Protocol-relative, backslash, and absolute URLs are rejected
 * so a crafted query cannot send the session somewhere else.
 */
export function safeReturnPath(raw: string | null): string | null {
  if (raw == null) return null;
  const value = raw.trim();
  if (value === '' || value.length > 2048) return null;
  if (!value.startsWith('/') || value.startsWith('//') || value.includes('\\')) return null;
  if (value.includes('\0') || value.includes('\r') || value.includes('\n') || value.includes('://'))
    return null;

  let url: URL;
  try {
    url = new URL(value, 'http://local.invalid');
  } catch {
    return null;
  }
  if (url.origin !== 'http://local.invalid') return null;
  if (url.username !== '' || url.password !== '') return null;
  if (!url.pathname.startsWith('/') || url.pathname.startsWith('//')) return null;
  return `${url.pathname}${url.search}${url.hash}`;
}

/** `next` wins when both are present. Missing or unsafe → `/`. */
export function safeReturnFromUrl(url: URL): string {
  return (
    safeReturnPath(url.searchParams.get('next')) ??
    safeReturnPath(url.searchParams.get('return')) ??
    '/'
  );
}

export function signinUrl(pathWithQuery: string): string {
  return `/signin?next=${encodeURIComponent(pathWithQuery)}`;
}
