import { error } from '@sveltejs/kit';
import { PHOTOS, NGC7000 } from '$lib/data/photos';
import type { PageServerLoad } from './$types';
import type { PhotoDetail } from '$lib/data/photos';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/** Minimal photo detail shape for gallery photos that lack rich EXIF data. */
function minimalDetail(
  target: string,
  integration: string,
  photographerName: string,
  slug: string,
  ratio: number
): PhotoDetail {
  return {
    slug,
    target,
    targetSubtitle: '',
    captured: '',
    camera: '',
    cameraSub: '',
    telescope: '',
    telescopeSub: '',
    mount: '',
    filters: '',
    exposure: '',
    exposureTotal: '',
    gain: '',
    ra: '',
    dec: '',
    field: '',
    pixelScale: '',
    publishedDate: '',
    photographer: {
      name: photographerName,
      initial: photographerName.charAt(0).toUpperCase(),
      frames: 0,
      bortle: 0,
      location: '',
      caption: ''
    },
    appreciations: 0,
    comments: 0,
    ratio,
    integration
  };
}

export const load: PageServerLoad = async ({ params, fetch }) => {
  const { slug } = params;

  // Canonical NGC 7000 placeholder
  if (slug === 'ngc-7000-north-america-nebula') {
    return { photo: NGC7000, isRich: true, thumbSrc1200: undefined };
  }

  // Real photo by UUID
  if (UUID_RE.test(slug)) {
    const res = await fetch(`${API}/api/photos/${slug}`);
    if (!res.ok) {
      if (res.status === 404) {
        error(404, 'Photo not found');
      }
      error(500, 'Failed to load photo');
    }
    const photo = (await res.json()) as {
      id: string;
      target: string | null;
      caption: string | null;
      camera: string | null;
      taken_at: string | null;
      width: number | null;
      height: number | null;
      iso: number | null;
      exposure_s: number | null;
    };

    const detail: PhotoDetail = {
      slug: photo.id,
      target: photo.target ?? 'Untitled',
      targetSubtitle: '',
      captured: photo.taken_at ?? '',
      camera: photo.camera ?? '',
      cameraSub: '',
      telescope: '',
      telescopeSub: '',
      mount: '',
      filters: '',
      exposure: photo.exposure_s != null ? `${photo.exposure_s} s` : '',
      exposureTotal: '',
      gain: photo.iso != null ? String(photo.iso) : '',
      ra: '',
      dec: '',
      field: '',
      pixelScale: '',
      publishedDate: '',
      photographer: {
        name: 'User',
        initial: 'U',
        frames: 0,
        bortle: 0,
        location: '',
        caption: photo.caption ?? '',
        captionShort: photo.caption ?? ''
      },
      appreciations: 0,
      comments: 0,
      ratio: photo.width && photo.height ? photo.width / photo.height : 1.5,
      integration: ''
    };

    return {
      photo: detail,
      isRich: false,
      thumbSrc1200: `${API}/api/photos/${photo.id}/thumb/1200`
    };
  }

  // Placeholder gallery photo by slug
  const match = PHOTOS.find((p) => p.slug === slug);
  if (!match) {
    error(404, 'Photo not found');
  }

  return {
    photo: minimalDetail(
      match.target,
      match.integration,
      match.photographer,
      match.slug,
      match.ratio
    ),
    isRich: false,
    thumbSrc1200: undefined
  };
};
