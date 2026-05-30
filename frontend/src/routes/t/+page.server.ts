import type { PageServerLoad } from './$types';
import { fetchTargetList } from '$lib/api/discoveryClient';
import { sizeBucketByKey } from '$lib/util/sizeBuckets';
import { error } from '@sveltejs/kit';

export const load: PageServerLoad = async ({ fetch, url }) => {
  const sort = url.searchParams.get('sort') ?? 'popular';
  const q = url.searchParams.get('q') ?? undefined;
  const object_type = url.searchParams.get('object_type') ?? undefined;
  const constellation = url.searchParams.get('constellation') ?? undefined;
  // Size filter is shared as a bucket key in the page URL; translate it to the
  // major-axis arcmin bounds the API expects.
  const size = url.searchParams.get('size') ?? undefined;
  const bucket = sizeBucketByKey(size);
  // Default to photographed targets only — the catalog holds ~12k OpenNGC
  // objects but most have no photos yet; leading with empty stubs is poor
  // UX and pollutes crawl. BUT "Optimal now" and the size buckets are
  // planning tools ("what should I shoot tonight"), which only make sense
  // against the full catalog — auto-include it for those. The `all` toggle
  // forces the full catalog for any sort.
  const explicitAll = url.searchParams.get('all') === '1';
  const planning = sort === 'optimal' || bucket !== undefined;
  const fullCatalog = explicitAll || planning;

  try {
    const initial = await fetchTargetList(fetch, {
      sort,
      ...(q !== undefined ? { q } : {}),
      ...(object_type !== undefined ? { object_type } : {}),
      ...(constellation !== undefined ? { constellation } : {}),
      ...(bucket?.min !== undefined ? { size_min: bucket.min } : {}),
      ...(bucket?.max !== undefined ? { size_max: bucket.max } : {}),
      ...(fullCatalog ? {} : { has_photos: true }),
      limit: 24
    });
    return { initial, sort, q, object_type, constellation, size, fullCatalog, planning };
  } catch (_e) {
    throw error(500, 'Failed to load targets');
  }
};
