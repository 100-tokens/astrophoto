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
  // The home-page pills and the sitemap link the hyphenated form
  // (/c/wide-field) while VALID_CATEGORIES holds the backend's underscore
  // form; the backend itself normalises hyphen↔underscore, so do the same
  // here instead of 404ing on /c/wide-field.
  const cat = params.cat.replace(/-/g, '_');
  if (!(VALID_CATEGORIES as readonly string[]).includes(cat)) {
    throw error(404, 'Category not found');
  }
  const sort = (url.searchParams.get('sort') ?? 'newest') as 'newest' | 'most-appreciated';
  // No `since` here: /api/categories/:cat does not implement it (only
  // /api/explore does), so forwarding it would silently no-op.
  try {
    const initial = await fetchCategoryPage(fetch, cat, { sort, limit: 24 });
    return { initial, sort };
  } catch (e) {
    if ((e as Error).message === 'not_found') throw error(404, 'Category not found');
    throw error(500, 'Failed to load category page');
  }
};
