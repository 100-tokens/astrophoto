import type { PageServerLoad } from './$types';
import { fetchSearch } from '$lib/api/discoveryClient';
import { error } from '@sveltejs/kit';

export const load: PageServerLoad = async ({ fetch, url }) => {
  const q = url.searchParams.get('q') ?? '';
  if (!q.trim()) {
    return { q, initial: null };
  }
  try {
    const initial = await fetchSearch(fetch, q);
    return { q, initial };
  } catch (_e) {
    throw error(500, 'Search failed');
  }
};
