import type { PageServerLoad } from './$types';
import { fetchExplore } from '$lib/api/discoveryClient';
import type { SiteStats } from '$lib/api/SiteStats';
import { error } from '@sveltejs/kit';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

// Query params come from anywhere (shared links, crawlers, stale
// bookmarks) — unknown values fall through to defaults instead of being
// cast-and-forwarded. The backend 400s on an invalid `since`, which this
// load used to translate into a 500 error page for the whole route.
const SORTS = ['newest', 'most-appreciated'] as const;
const SINCES = ['24h', '7d', '30d', 'all'] as const;
const CATEGORIES = [
  'dso',
  'planetary',
  'lunar',
  'solar',
  'wide_field',
  'nightscape',
  'other'
] as const;

function pick<T extends string>(raw: string | null, allowed: readonly T[], fallback: T): T {
  return allowed.includes(raw as T) ? (raw as T) : fallback;
}

export const load: PageServerLoad = async ({ fetch, url, locals }) => {
  const sort = pick(url.searchParams.get('sort'), SORTS, 'newest');
  const since = pick(url.searchParams.get('since'), SINCES, '7d');
  const categoryParam = url.searchParams.get('category');
  const category =
    categoryParam !== null && (CATEGORIES as readonly string[]).includes(categoryParam)
      ? categoryParam
      : undefined;
  // 'Following only' needs a session: anonymous visitors landing on a
  // shared ?following=true URL used to get an empty grid with a
  // "be the first to publish" message and the responsible pill hidden
  // (it only renders for authed users). Strip it instead — they see the
  // full feed.
  const following = url.searchParams.get('following') === 'true' && locals.user !== null;

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
