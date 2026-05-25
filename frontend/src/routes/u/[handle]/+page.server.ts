import { error, redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { fetchPublicProfile, fetchPhotosFeed } from '$lib/api/profileClient';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

// RFC 4122 UUID. Used to spot when someone fed `/u/<id>` instead of
// `/u/<handle>` so we can redirect to the canonical URL.
const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export const load: PageServerLoad = async ({ params, fetch, locals }) => {
  const { handle } = params;

  // If the slug looks like a UUID, this is almost certainly a user id pasted
  // into the URL rather than a real handle. Look the user up by id and 301
  // to their canonical handle URL instead of throwing 404.
  if (UUID_RE.test(handle)) {
    const r = await fetch(`${API}/api/users/${handle}`);
    if (r.ok) {
      const { handle: target } = (await r.json()) as { handle: string };
      throw redirect(301, `/u/${target}`);
    }
    // Fall through to the not-found path below if the id matches no user.
  }

  // The gallery feed only needs `handle` (a route param), not the profile, so
  // start it now to run concurrently with the profile lookup rather than after
  // it. If the profile turns out to be a redirect/404, this fetch is discarded.
  const feedP = fetchPhotosFeed(fetch, handle, { limit: 24 }).catch(() => null);

  let profile;
  try {
    profile = await fetchPublicProfile(fetch, handle);
  } catch (e) {
    if ((e as Error).message === 'not_found') {
      // Check redirect history — handle may have been renamed.
      const rRes = await fetch(`${API}/api/handles/redirect/${handle}`);
      if (rRes.ok) {
        const { handle: target } = (await rRes.json()) as { handle: string };
        throw redirect(301, `/u/${target}`);
      }
      throw error(404, 'No photographer here.');
    }
    throw error(500, 'Profile lookup failed');
  }

  // First page of the gallery — SSR'd so the hero gallery has content on first paint.
  let firstPage: {
    photos: import('$lib/api/GalleryPhoto').GalleryPhoto[];
    next_cursor: string | null;
  } | null = null;
  const page = await feedP;
  if (page) {
    firstPage = { photos: page.photos, next_cursor: page.next_cursor ?? null };
  }

  const isSelf = locals.user?.id === profile.id;
  const viewMode: 'visitor' | 'owner' | 'admin' = isSelf ? 'owner' : 'visitor';

  return { profile, firstPage, viewMode };
};
