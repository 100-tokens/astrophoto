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

  // The processing report (XISF only) and the "more from this photographer"
  // feed are independent of each other — run them concurrently so the detail
  // page's TTFB is one round-trip instead of two. XISF gating skips the
  // processing request entirely for non-XISF photos.
  const processingP: Promise<import('$lib/api/types').ProcessingReport | null> =
    photo.mime === 'application/x-xisf'
      ? fetch(`${API}/api/photos/${id}/processing`).then((pr) => (pr.ok ? pr.json() : null))
      : Promise.resolve(null);
  const feedP = fetchPhotosFeed(fetch, handle, { limit: 24 }).catch(() => null);

  const [processing, feed] = await Promise.all([processingP, feedP]);

  // For "More from this photographer" + prev/next.
  let morePhotos: import('$lib/api/GalleryPhoto').GalleryPhoto[] = [];
  let prevShortid: string | null = null;
  let nextShortid: string | null = null;
  if (feed) {
    const others = feed.photos.filter((p) => p.id !== photo.id);
    morePhotos = others.slice(0, 4);
    const idx = feed.photos.findIndex((p) => p.id === photo.id);
    if (idx > 0) prevShortid = feed.photos[idx - 1]?.short_id ?? null;
    if (idx >= 0 && idx < feed.photos.length - 1)
      nextShortid = feed.photos[idx + 1]?.short_id ?? null;
  }

  return { photo, handle, morePhotos, prevShortid, nextShortid, processing };
};
