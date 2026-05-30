import type { PageServerLoad } from './$types';
import { fetchExplore } from '$lib/api/discoveryClient';
import type { SiteStats } from '$lib/api/SiteStats';
import { error } from '@sveltejs/kit';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ fetch, url }) => {
  const sort = (url.searchParams.get('sort') ?? 'newest') as 'newest' | 'most-appreciated';
  const since = (url.searchParams.get('since') ?? '7d') as '24h' | '7d' | '30d' | 'all';
  const categoryParam = url.searchParams.get('category');
  const category = categoryParam !== null ? categoryParam : undefined;
  const following = url.searchParams.get('following') === 'true';

  // Site-wide frame total for the eyebrow — non-fatal, runs in parallel with
  // the feed so it never adds a serial round-trip to SSR.
  const statsPromise = fetch(`${API}/api/site/stats`)
    .then(async (r) => (r.ok ? ((await r.json()) as SiteStats) : null))
    .catch(() => null);

  try {
    const [initial, stats] = await Promise.all([
      fetchExplore(fetch, {
        sort,
        since,
        ...(category !== undefined ? { category } : {}),
        following,
        limit: 24
      }),
      statsPromise
    ]);
    return { initial, sort, since, category, following, totalFrames: stats?.frames ?? null };
  } catch (_e) {
    throw error(500, 'Failed to load explore feed');
  }
};
