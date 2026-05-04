import type { PageServerLoad } from './$types';
import { fetchExplore } from '$lib/api/discoveryClient';
import { error } from '@sveltejs/kit';

export const load: PageServerLoad = async ({ fetch, url }) => {
  const sort = (url.searchParams.get('sort') ?? 'newest') as 'newest' | 'most-appreciated';
  const since = (url.searchParams.get('since') ?? '7d') as '24h' | '7d' | '30d' | 'all';
  const categoryParam = url.searchParams.get('category');
  const category = categoryParam !== null ? categoryParam : undefined;
  const following = url.searchParams.get('following') === 'true';
  try {
    const initial = await fetchExplore(fetch, {
      sort,
      since,
      ...(category !== undefined ? { category } : {}),
      following,
      limit: 24
    });
    return { initial, sort, since, category, following };
  } catch (_e) {
    throw error(500, 'Failed to load explore feed');
  }
};
