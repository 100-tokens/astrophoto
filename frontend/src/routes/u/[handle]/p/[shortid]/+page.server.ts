import { error } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import { fetchPhotosFeed } from '$lib/api/profileClient';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ params, fetch }) => {
  const { handle, shortid } = params;

  const r = await fetch(`${API}/api/photos/by-permalink/${handle}/${shortid}`);
  if (!r.ok) throw error(404, 'Photo not found');
  const { id } = (await r.json()) as { id: string };

  const photoR = await fetch(`${API}/api/photos/${id}`);
  if (!photoR.ok) throw error(404, 'Photo not found');

  const photo = await photoR.json();

  // For "More from this photographer" + prev/next: best-effort feed fetch.
  let morePhotos: import('$lib/api/GalleryPhoto').GalleryPhoto[] = [];
  let prevShortid: string | null = null;
  let nextShortid: string | null = null;
  try {
    const feed = await fetchPhotosFeed(fetch, handle, { limit: 24 });
    const others = feed.photos.filter((p) => p.id !== photo.id);
    morePhotos = others.slice(0, 4);
    const idx = feed.photos.findIndex((p) => p.id === photo.id);
    if (idx > 0) prevShortid = feed.photos[idx - 1]?.short_id ?? null;
    if (idx >= 0 && idx < feed.photos.length - 1) nextShortid = feed.photos[idx + 1]?.short_id ?? null;
  } catch (_e) {
    // ignore — fall back to empty values
  }

  return { photo, handle, morePhotos, prevShortid, nextShortid };
};
