import type { PageServerLoad } from './$types';
import { fetchTargetList } from '$lib/api/discoveryClient';
import { error } from '@sveltejs/kit';

export const load: PageServerLoad = async ({ fetch, url }) => {
  const sort = url.searchParams.get('sort') ?? 'popular';
  const q = url.searchParams.get('q') ?? undefined;
  const object_type = url.searchParams.get('object_type') ?? undefined;
  const constellation = url.searchParams.get('constellation') ?? undefined;

  try {
    const initial = await fetchTargetList(fetch, {
      sort,
      ...(q !== undefined ? { q } : {}),
      ...(object_type !== undefined ? { object_type } : {}),
      ...(constellation !== undefined ? { constellation } : {}),
      limit: 24
    });
    return { initial, sort, q, object_type, constellation };
  } catch (_e) {
    throw error(500, 'Failed to load targets');
  }
};
