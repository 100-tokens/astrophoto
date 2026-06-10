import type { PageServerLoad } from './$types';
import { fetchEquipmentPage } from '$lib/api/discoveryClient';
import type { EquipmentItemDetail } from '$lib/api/EquipmentItemDetail';
import { error, redirect } from '@sveltejs/kit';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ fetch, params, locals }) => {
  // Catalog spec editing is admin-only (the backend PATCH is
  // AdminUser-guarded since the 2026-06 audit — the "phase 2" role gate
  // the previous comment promised). Mirrors /admin's layout guard so
  // non-admins get a clear error instead of a dead form + 403 on save.
  if (!locals.user) {
    throw redirect(303, `/signin?next=/equip/${params.kind}/${params.slug}/edit`);
  }
  if (!locals.user.isAdmin) {
    throw error(403, 'Catalog editing requires an admin account');
  }

  let initial;
  try {
    initial = await fetchEquipmentPage(fetch, params.kind, params.slug, {
      sort: 'newest',
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
