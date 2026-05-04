import type { PageServerLoad } from './$types';
import { fetchCategoryPage } from '$lib/api/discoveryClient';
import { error } from '@sveltejs/kit';

const VALID_CATEGORIES = [
  'dso',
  'planetary',
  'lunar',
  'solar',
  'wide_field',
  'nightscape',
  'other'
] as const;

export const load: PageServerLoad = async ({ fetch, url, params }) => {
  if (!(VALID_CATEGORIES as readonly string[]).includes(params.cat)) {
    throw error(404, 'Category not found');
  }
  const sort = (url.searchParams.get('sort') ?? 'newest') as 'newest' | 'most-appreciated';
  const since = (url.searchParams.get('since') ?? 'all') as '24h' | '7d' | '30d' | 'all';
  try {
    const initial = await fetchCategoryPage(fetch, params.cat, { sort, since, limit: 24 });
    return { initial, sort, since };
  } catch (e) {
    if ((e as Error).message === 'not_found') throw error(404, 'Category not found');
    throw error(500, 'Failed to load category page');
  }
};
