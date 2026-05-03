import { error, redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { fetchPublicProfile, fetchPhotosFeed } from '$lib/api/profileClient';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ params, fetch, locals }) => {
  const { handle } = params;

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
  try {
    const page = await fetchPhotosFeed(fetch, handle, { limit: 24 });
    firstPage = { photos: page.photos, next_cursor: page.next_cursor ?? null };
  } catch (_e) {
    // Backend hiccup — proceed without SSR'd photos; PhotoGrid will fetch on mount.
  }

  const isSelf = locals.user?.id === profile.id;
  const viewMode: 'visitor' | 'owner' | 'admin' = isSelf ? 'owner' : 'visitor';

  return { profile, firstPage, viewMode };
};
