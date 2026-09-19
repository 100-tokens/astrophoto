import { cdn } from '$lib/cdn';
import type { PageServerLoad } from './$types';
import type { SiteStats } from '$lib/api/SiteStats';
import type { DiscoveryPage } from '$lib/api/DiscoveryPage';
import type { DiscoveryPhoto } from '$lib/api/DiscoveryPhoto';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

// Home and Explore share the discovery feed. `since=all` is the whole
// archive (the explore handler treats a missing window the same way).
// The list endpoint does not carry a handle, so permalinks come from here.
async function loadExplore(
  fetch: typeof globalThis.fetch,
  cookie: string,
  following: boolean
): Promise<DiscoveryPhoto[]> {
  const params = new URLSearchParams({ limit: '24', since: 'all' });
  if (following) params.set('following', 'true');
  try {
    const res = await fetch(`${API}/api/explore?${params}`, {
      headers: cookie ? { Cookie: cookie } : {}
    });
    if (!res.ok) return [];
    const body = (await res.json()) as DiscoveryPage;
    return body.photos;
  } catch {
    return [];
  }
}

function permalink(p: DiscoveryPhoto): string {
  return `/u/${p.author_handle}/p/${p.short_id}`;
}

export const load: PageServerLoad = async ({ fetch, locals, request }) => {
  const statsPromise = fetch(`${API}/api/site/stats`)
    .then(async (r) => (r.ok ? ((await r.json()) as SiteStats) : null))
    .catch(() => null);

  const cookie = request.headers.get('cookie') ?? '';
  let realPhotos: DiscoveryPhoto[] = [];
  // Only label the feed as follows when that query actually returned frames.
  // An empty follows list falls through to the public archive.
  let fromFollows = false;

  if (locals.user) {
    const followed = await loadExplore(fetch, cookie, true);
    if (followed.length > 0) {
      realPhotos = followed;
      fromFollows = true;
    }
  }

  if (realPhotos.length === 0) {
    realPhotos = await loadExplore(fetch, '', false);
    fromFollows = false;
  }

  const following_count = locals.user?.following_ids?.length ?? 0;
  const stats = await statsPromise;

  if (realPhotos.length > 0) {
    const [hero, ...rest] = realPhotos as [DiscoveryPhoto, ...DiscoveryPhoto[]];
    return {
      heroPhoto: {
        target: hero.target,
        original_name: hero.original_name,
        integration: '',
        photographer: hero.author_display_name
      },
      heroSrc: cdn(hero.id, { w: 1200 }),
      heroHref: permalink(hero),
      photos: rest.map((p) => ({
        slug: p.id,
        href: permalink(p),
        target: p.target,
        original_name: p.original_name,
        ratio: p.width && p.height ? p.width / p.height : 1.5,
        integration: '',
        photographer: p.author_display_name,
        photographerSlug: p.author_handle,
        camera: '',
        thumbSrc: cdn(p.id, { w: 400 })
      })),
      isReal: true,
      fromFollows,
      following_count,
      stats
    };
  }

  return {
    heroPhoto: null,
    heroSrc: undefined,
    heroHref: null,
    photos: [],
    isReal: false,
    fromFollows: false,
    following_count,
    stats
  };
};
