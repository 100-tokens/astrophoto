import { error, redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import type { User, Photo } from '$lib/data/photos';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

interface UserByHandle {
  id: string;
  handle: string;
  display_name: string;
  created_at: string;
  photo_count: number;
}

export const load: PageServerLoad = async ({ params, fetch, locals }) => {
  const { handle } = params;

  // 1) Try to resolve the current handle.
  const userRes = await fetch(`${API}/api/users/by-handle/${handle}`);
  if (userRes.status === 404) {
    // 2) Check redirect history — handle may have been renamed.
    const rRes = await fetch(`${API}/api/handles/redirect/${handle}`);
    if (rRes.ok) {
      const { handle: target } = (await rRes.json()) as { handle: string };
      throw redirect(301, `/u/${target}`);
    }
    throw error(404, 'No photographer here.');
  }
  if (!userRes.ok) throw error(500, 'Lookup failed');

  const u = (await userRes.json()) as UserByHandle;

  // If the viewer is themselves the owner, prefer locals.user displayName
  // (most up-to-date, avoids a stale cache).
  let displayName = u.display_name;
  if (locals.user?.id === u.id) {
    displayName = locals.user.displayName;
  }

  let followerCount = 0;
  try {
    const res = await fetch(`${API}/api/users/${u.id}/followers/count`);
    if (res.ok) {
      const body = (await res.json()) as { count: number };
      followerCount = body.count;
    }
  } catch {
    // ignore — keep default 0
  }

  type ProfilePhoto = Omit<Photo, 'target'> & { target: string | null };
  let photos: ProfilePhoto[] = [];
  try {
    const res = await fetch(`${API}/api/photos?owner_id=${u.id}&limit=24`);
    if (res.ok) {
      const body = (await res.json()) as {
        photos: Array<{
          id: string;
          target: string | null;
          width: number | null;
          height: number | null;
        }>;
      };
      photos = body.photos.map((p) => ({
        // slug carries the UUID for now; Task 52 will switch to short_id once
        // /api/photos surfaces that field in the list response.
        slug: p.id,
        target: p.target,
        ratio: p.width && p.height ? p.width / p.height : 1.5,
        integration: '',
        photographer: '',
        photographerSlug: '',
        camera: '',
        thumbSrc: `${API}/api/photos/${p.id}/thumb/400`
      }));
    }
  } catch {
    // backend down — return empty gallery
  }

  const memberSince = new Date(u.created_at).getFullYear().toString();
  const parts = displayName.split(' ');
  const firstName = parts[0] ?? displayName;
  const surnameItalic = parts.slice(1).join(' ') || displayName;

  const profile: User = {
    username: u.handle,
    displayName,
    firstName,
    surnameItalic,
    initial: displayName[0]?.toUpperCase() ?? 'U',
    about: '',
    frames: u.photo_count,
    integrationTotal: '—',
    followers: followerCount,
    collections: 0,
    lat: '—',
    long: '—',
    bortle: 0,
    sqm: 0,
    equipment: { scope: '—', camera: '—', mount: '—', filters: '—' },
    memberSince
  };

  // isFollowing and isSelf are keyed on UUID, not the handle.
  const isFollowing = locals.user?.following_ids?.includes(u.id) ?? false;
  const isSelf = locals.user?.id === u.id;

  // Field name 'profile' (not 'user') to avoid colliding with the layout's
  // `data.user` (the auth state). Layout-level `user` must keep flowing
  // through to AppHeader's $app/state read.
  // userId carries the UUID separately so FollowButton can call the follow API.
  return { profile, photos, userId: u.id, isFollowing, isSelf };
};
