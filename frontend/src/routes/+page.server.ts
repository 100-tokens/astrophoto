import { PHOTOS, NGC7000 } from '$lib/data/photos';
import type { PageServerLoad } from './$types';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ fetch }) => {
  let realPhotos: Array<{
    id: string;
    target: string | null;
    width: number | null;
    height: number | null;
  }> = [];
  try {
    const res = await fetch(`${API}/api/photos?limit=24`);
    if (res.ok) {
      const body = (await res.json()) as { photos: typeof realPhotos };
      realPhotos = body.photos;
    }
  } catch {
    // backend down — fall through to placeholders
  }

  // If we have real photos, build a gallery from them. Otherwise keep
  // the placeholder demo content for a non-empty landing.
  if (realPhotos.length > 0) {
    const [hero, ...rest] = realPhotos as [(typeof realPhotos)[0], ...typeof realPhotos];
    return {
      heroPhoto: {
        target: hero.target ?? 'Untitled',
        integration: '',
        photographer: ''
      },
      heroSrc: `${API}/api/photos/${hero.id}/thumb/1200`,
      photos: rest.map((p) => ({
        slug: p.id,
        target: p.target ?? 'Untitled',
        ratio: p.width && p.height ? p.width / p.height : 1.5,
        integration: '',
        photographer: '',
        photographerSlug: '',
        camera: '',
        thumbSrc: `${API}/api/photos/${p.id}/thumb/400`
      })),
      isReal: true
    };
  }

  return {
    heroPhoto: {
      target: NGC7000.target,
      integration: NGC7000.integration,
      photographer: NGC7000.photographer.name
    },
    heroSrc: undefined,
    photos: PHOTOS.slice(0, 12).map((p) => ({ ...p, thumbSrc: undefined })),
    isReal: false
  };
};
