import { env } from '$env/dynamic/public';

const BASE = env.PUBLIC_CDN_BASE_URL ?? '/cdn';

export type Transform = {
  w?: number;
  h?: number;
  fit?: 'cover' | 'contain';
  q?: number;
  fm?: 'auto' | 'jpg' | 'webp';
};

export function cdn(photoId: string, t: Transform = {}): string {
  const url = new URL(`${BASE}/img/${photoId}`, 'http://placeholder');
  if (t.w)   url.searchParams.set('w',  String(t.w));
  if (t.h)   url.searchParams.set('h',  String(t.h));
  if (t.fit) url.searchParams.set('fit', t.fit);
  if (t.q)   url.searchParams.set('q',  String(t.q));
  if (t.fm)  url.searchParams.set('fm', t.fm);
  // strip placeholder origin
  return url.pathname + url.search;
}

export function srcset(photoId: string, widths: number[], t: Omit<Transform, 'w'> = {}): string {
  return widths.map(w => `${cdn(photoId, { ...t, w })} ${w}w`).join(', ');
}
