import { PHOTOS, NGC7000 } from '$lib/data/photos';
import { cdn } from '$lib/cdn';
import type { PageServerLoad } from './$types';
import type { SiteStats } from '$lib/api/SiteStats';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

type RealPhoto = {
  id: string;
  target: string | null;
  original_name: string;
  width: number | null;
  height: number | null;
  owner_id?: string;
};

export const load: PageServerLoad = async ({ fetch, locals, request }) => {
  let realPhotos: RealPhoto[] = [];

  // Fire site stats in parallel — non-fatal, falls back to nulls if backend
  // hiccups so the home still renders.
  const statsPromise = fetch(`${API}/api/site/stats`)
    .then(async (r) => (r.ok ? ((await r.json()) as SiteStats) : null))
    .catch(() => null);

  // 1. Authenticated user with follows: try the personalised feed first.
  if (locals.user) {
    try {
      const cookie = request.headers.get('cookie') ?? '';
      const res = await fetch(`${API}/api/photos?following=true`, {
        headers: cookie ? { Cookie: cookie } : {}
      });
      if (res.ok) {
        const body = (await res.json()) as { photos: RealPhoto[] };
        realPhotos = body.photos;
      }
    } catch {
      // ignore — fall through to public list
    }
  }

  // 2. Anonymous, OR auth user follows nobody, OR follows-with-no-photos:
  //    show the public feed.
  if (realPhotos.length === 0) {
    try {
      const res = await fetch(`${API}/api/photos?limit=24`);
      if (res.ok) {
        const body = (await res.json()) as { photos: RealPhoto[] };
        realPhotos = body.photos;
      }
    } catch {
      // backend down — fall back to placeholder demo content
    }
  }

  // If we have real photos, build a gallery from them. Otherwise keep
  // the placeholder demo content for a non-empty landing.
  const following_count = locals.user?.following_ids?.length ?? 0;
  const stats = await statsPromise;

  if (realPhotos.length > 0) {
    const [hero, ...rest] = realPhotos as [RealPhoto, ...RealPhoto[]];
    return {
      heroPhoto: {
        target: hero.target,
        original_name: hero.original_name,
        integration: '',
        photographer: ''
      },
      heroSrc: cdn(hero.id, { w: 1200 }),
      photos: rest.map((p) => ({
        slug: p.id,
        target: p.target,
        original_name: p.original_name,
        ratio: p.width && p.height ? p.width / p.height : 1.5,
        integration: '',
        photographer: '',
        photographerSlug: '',
        camera: '',
        thumbSrc: cdn(p.id, { w: 400 })
      })),
      isReal: true,
      following_count,
      stats
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
    isReal: false,
    following_count,
    stats
  };
};
