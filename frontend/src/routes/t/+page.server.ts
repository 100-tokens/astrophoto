import type { PageServerLoad } from './$types';
import { fetchTargetList } from '$lib/api/discoveryClient';
import { error } from '@sveltejs/kit';

export const load: PageServerLoad = async ({ fetch, url }) => {
  const sort = url.searchParams.get('sort') ?? 'popular';
  const q = url.searchParams.get('q') ?? undefined;
  const object_type = url.searchParams.get('object_type') ?? undefined;
  const constellation = url.searchParams.get('constellation') ?? undefined;
  // Default to photographed targets only — the catalog holds ~12k OpenNGC
  // objects but most have no photos yet; leading with empty stubs is poor
  // UX and pollutes crawl. `?all=1` opts into the full catalog (the search
  // box reaches every object via its own query path regardless).
  const showAll = url.searchParams.get('all') === '1';

  try {
    const initial = await fetchTargetList(fetch, {
      sort,
      ...(q !== undefined ? { q } : {}),
      ...(object_type !== undefined ? { object_type } : {}),
      ...(constellation !== undefined ? { constellation } : {}),
      ...(showAll ? {} : { has_photos: true }),
      limit: 24
    });
    return { initial, sort, q, object_type, constellation, showAll };
  } catch (_e) {
    throw error(500, 'Failed to load targets');
  }
};
