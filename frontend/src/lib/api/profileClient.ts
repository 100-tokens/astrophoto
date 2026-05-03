// API client wrappers for the P2 profile endpoints.
// `client.ts` covers the legacy/photo APIs; this module is the surface
// the hero page, profile editor, cover picker, and featured controls use.

import type { Profile } from './Profile';
import type { PublicProfile } from './PublicProfile';
import type { GalleryPage } from './GalleryPage';
import type { SocialLink } from './SocialLink';

const API_BASE: string = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '';

type FetchFn = typeof fetch;

export async function fetchOwnerProfile(f: FetchFn): Promise<Profile> {
  const r = await f(`${API_BASE}/api/me/profile`, { credentials: 'include' });
  if (!r.ok) throw new Error(`fetchOwnerProfile ${r.status}`);
  return (await r.json()) as Profile;
}

export interface ProfilePatchBody {
  display_name?: string | null;
  tagline?: string | null;
  bio_html?: string | null;
  equipment?: Profile['equipment'];
  location?: Profile['location'];
  social_links?: SocialLink[];
}

export async function patchOwnerProfile(f: FetchFn, body: ProfilePatchBody): Promise<void> {
  const r = await f(`${API_BASE}/api/me/profile`, {
    method: 'PATCH',
    credentials: 'include',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body)
  });
  if (!r.ok) throw new Error(`patchOwnerProfile ${r.status}`);
}

export async function fetchPublicProfile(f: FetchFn, handle: string): Promise<PublicProfile> {
  const r = await f(`${API_BASE}/api/users/by-handle/${handle}/profile`);
  if (r.status === 404) throw new Error('not_found');
  if (!r.ok) throw new Error(`fetchPublicProfile ${r.status}`);
  return (await r.json()) as PublicProfile;
}

export async function fetchPhotosFeed(
  f: FetchFn,
  handle: string,
  opts: { cursor?: string; sort?: 'newest' | 'popular'; limit?: number } = {}
): Promise<GalleryPage> {
  const params = new URLSearchParams();
  if (opts.cursor) params.set('cursor', opts.cursor);
  if (opts.sort) params.set('sort', opts.sort);
  if (opts.limit !== undefined) params.set('limit', String(opts.limit));
  const qs = params.toString();
  const url = `${API_BASE}/api/users/by-handle/${handle}/photos${qs ? `?${qs}` : ''}`;
  const r = await f(url);
  if (!r.ok) throw new Error(`fetchPhotosFeed ${r.status}`);
  return (await r.json()) as GalleryPage;
}

export async function setCover(f: FetchFn, photoId: string | null): Promise<void> {
  const r = await f(`${API_BASE}/api/me/cover`, {
    method: 'POST',
    credentials: 'include',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ photo_id: photoId })
  });
  if (!r.ok) throw new Error(`setCover ${r.status}`);
}

export async function pinFeatured(f: FetchFn, photoId: string): Promise<void> {
  const r = await f(`${API_BASE}/api/me/featured/${photoId}`, {
    method: 'POST',
    credentials: 'include'
  });
  if (!r.ok) throw new Error(`pinFeatured ${r.status}`);
}

export async function unpinFeatured(f: FetchFn, photoId: string): Promise<void> {
  const r = await f(`${API_BASE}/api/me/featured/${photoId}`, {
    method: 'DELETE',
    credentials: 'include'
  });
  if (!r.ok) throw new Error(`unpinFeatured ${r.status}`);
}

export async function reorderFeatured(f: FetchFn, photoIds: string[]): Promise<void> {
  const r = await f(`${API_BASE}/api/me/featured/order`, {
    method: 'PATCH',
    credentials: 'include',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ photo_ids: photoIds })
  });
  if (!r.ok) throw new Error(`reorderFeatured ${r.status}`);
}
