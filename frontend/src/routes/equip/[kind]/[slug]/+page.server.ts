import type { PageServerLoad } from './$types';
import { fetchEquipmentPage } from '$lib/api/discoveryClient';
import type { EquipmentItemDetail } from '$lib/api/EquipmentItemDetail';
import { error } from '@sveltejs/kit';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

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

    // Fetch full item detail (specs) using the id from the equipment meta.
    // The endpoint is now public (no auth required).
    let item: EquipmentItemDetail | null = null;
    try {
      const itemR = await fetch(`${API}/api/equipment/items/${initial.equipment.id}`);
      if (itemR.ok) {
        item = (await itemR.json()) as EquipmentItemDetail;
      }
    } catch (_e) {
      // Non-fatal: specs header will show "+ Add specs" fallback.
    }

    return { initial, sort, since, category, item };
  } catch (e) {
    if ((e as Error).message === 'not_found') throw error(404, 'Equipment not found');
    throw error(500, 'Failed to load equipment page');
  }
};
