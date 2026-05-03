import type { PageServerLoad } from './$types';
import { fetchEquipmentPage } from '$lib/api/discoveryClient';
import { error } from '@sveltejs/kit';

export const load: PageServerLoad = async ({ fetch, url, params }) => {
  const sort = (url.searchParams.get('sort') ?? 'newest') as 'newest' | 'most-appreciated';
  const since = (url.searchParams.get('since') ?? 'all') as '24h' | '7d' | '30d' | 'all';
  const categoryParam = url.searchParams.get('category');
  const category = categoryParam !== null ? categoryParam : undefined;
  try {
    const initial = await fetchEquipmentPage(fetch, params.kind, params.slug, {
      sort,
      since,
      ...(category !== undefined ? { category } : {}),
      limit: 24
    });
    return { initial, sort, since, category };
  } catch (e) {
    if ((e as Error).message === 'not_found') throw error(404, 'Equipment not found');
    throw error(500, 'Failed to load equipment page');
  }
};
