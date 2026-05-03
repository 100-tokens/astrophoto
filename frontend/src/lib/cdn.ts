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
  const params = new URLSearchParams();
  if (t.w) params.set('w', String(t.w));
  if (t.h) params.set('h', String(t.h));
  if (t.fit) params.set('fit', t.fit);
  if (t.q) params.set('q', String(t.q));
  if (t.fm) params.set('fm', t.fm);
  const qs = params.toString();
  return qs ? `${BASE}/img/${photoId}?${qs}` : `${BASE}/img/${photoId}`;
}

export function srcset(photoId: string, widths: number[], t: Omit<Transform, 'w'> = {}): string {
  return widths.map((w) => `${cdn(photoId, { ...t, w })} ${w}w`).join(', ');
}
