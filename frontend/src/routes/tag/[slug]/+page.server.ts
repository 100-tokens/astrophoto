import type { PageServerLoad } from './$types';
import { fetchTagPage } from '$lib/api/discoveryClient';
import { error } from '@sveltejs/kit';

export const load: PageServerLoad = async ({ fetch, url, params }) => {
  const sort = (url.searchParams.get('sort') ?? 'newest') as 'newest' | 'most-appreciated';
  // No `since` here: /api/tags/:slug does not implement it (only
  // /api/explore does), so forwarding it would silently no-op.
  const categoryParam = url.searchParams.get('category');
  const category = categoryParam !== null ? categoryParam : undefined;
  try {
    const initial = await fetchTagPage(fetch, params.slug, {
      sort,
      ...(category !== undefined ? { category } : {}),
      limit: 24
    });
    return { initial, sort, category };
  } catch (e) {
    if ((e as Error).message === 'not_found') throw error(404, 'Tag not found');
    throw error(500, 'Failed to load tag page');
  }
};
