import type { PageServerLoad } from './$types';
import { fetchEquipmentPage } from '$lib/api/discoveryClient';
import type { EquipmentItemDetail } from '$lib/api/EquipmentItemDetail';
import { error, redirect } from '@sveltejs/kit';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ fetch, params, cookies }) => {
  // Auth is required to edit catalog specs. Phase 1 lets any signed-in
  // user write; phase 2 will gate by role.
  const session = cookies.get('session') ?? cookies.get('__Host-session');
  if (!session) throw redirect(303, `/signin?next=/equip/${params.kind}/${params.slug}/edit`);

  let initial;
  try {
    initial = await fetchEquipmentPage(fetch, params.kind, params.slug, {
      sort: 'newest',
      since: 'all',
      limit: 1
    });
  } catch (e) {
    if ((e as Error).message === 'not_found') throw error(404, 'Equipment not found');
    throw error(500, 'Failed to load equipment page');
  }

  const itemR = await fetch(`${API}/api/equipment/items/${initial.equipment.id}`);
  if (!itemR.ok) throw error(itemR.status, 'Item not found');
  const item = (await itemR.json()) as EquipmentItemDetail;

  return { item, equipment: initial.equipment };
};
