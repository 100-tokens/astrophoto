import type { PageServerLoad } from './$types';
import { fetchEquipmentItem } from '$lib/api/adminClient';
import { error } from '@sveltejs/kit';

export const load: PageServerLoad = async ({ fetch, params }) => {
  try {
    const item = await fetchEquipmentItem(fetch, params.id);
    return { item };
  } catch (e) {
    if ((e as Error).message.includes('404')) throw error(404, 'Equipment item not found');
    throw error(500, 'Failed to load equipment item');
  }
};
