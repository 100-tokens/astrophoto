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
  // Celestial objects identified by plate-solve. Only fetched when the
  // photo has a solve (no point asking otherwise). Silent failure → empty
  // array; the overlay simply does not render.
  const celestialP: Promise<import('$lib/api/CelestialObject').CelestialObject[]> =
    photo.ra_deg != null
      ? fetch(`${API}/api/photos/${id}/celestial-objects`)
          .then((r) => (r.ok ? r.json() : { objects: [] }))
          .then((j: { objects?: import('$lib/api/CelestialObject').CelestialObject[] }) => j.objects ?? [])
          .catch(() => [])
      : Promise.resolve([]);
  // The pixel scale + rotation needed by the WCS projection live on the
  // platesolve-status endpoint, not on PhotoDetail. Fetch in parallel.
  const platesolveP: Promise<import('$lib/api/PlatesolveStatus').PlatesolveStatus | null> =
    photo.ra_deg != null
      ? fetch(`${API}/api/photos/${id}/platesolve-status`)
          .then((r) => (r.ok ? r.json() : null))
          .catch(() => null)
      : Promise.resolve(null);

  const [processing, feed, celestialObjects, platesolveStatus] = await Promise.all([
    processingP,
    feedP,
    celestialP,
    platesolveP
  ]);

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

  return {
    photo,
    handle,
    morePhotos,
    prevShortid,
    nextShortid,
    processing,
    celestialObjects,
    platesolveStatus
  };
};
