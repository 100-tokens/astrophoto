import { error, redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import type { SetupDetail } from '$lib/api/SetupDetail';
import type { EquipmentItemDetail } from '$lib/api/EquipmentItemDetail';
import type { EquipmentSpecsPayload } from '$lib/api/EquipmentSpecsPayload';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

type ItemPrefill = {
  id: string;
  display_name: string;
  specs: EquipmentSpecsPayload | null;
};

type EditPrefill = {
  optical_tube?: ItemPrefill;
  main_camera?: ItemPrefill;
  mount?: ItemPrefill;
  focal_modifier?: ItemPrefill;
  filters?: Array<{
    id: string;
    display_name: string;
    filter_type: null;
    bandwidth_nm: null;
    position: number;
  }>;
};

const ROLE_KEYS = ['optical_tube', 'main_camera', 'mount', 'focal_modifier'] as const;

export const load: PageServerLoad = async ({ params, locals, fetch, cookies }) => {
  if (!locals.user) redirect(303, '/signin');

  const cookie = cookies
    .getAll()
    .map((c) => `${c.name}=${c.value}`)
    .join('; ');

  const r = await fetch(`${API}/api/equipment/setups/${params.id}`, {
    headers: { Cookie: cookie }
  });
  if (r.status === 404) error(404, 'Setup not found');
  if (!r.ok) error(500, 'Backend error');
  const setup: SetupDetail = await r.json();

  // Fetch full item details (with specs) for each non-filter role.
  // ≤4 items; run in parallel.
  const roleItems = setup.items.filter((it): it is (typeof setup.items)[number] =>
    (ROLE_KEYS as readonly string[]).includes(it.role)
  );

  const detailResults = await Promise.all(
    roleItems.map(async (it) => {
      const dr = await fetch(`${API}/api/equipment/items/${it.item.id}`, {
        headers: { Cookie: cookie }
      });
      if (!dr.ok) {
        return {
          role: it.role,
          prefill: {
            id: it.item.id,
            display_name: it.item.display_name,
            specs: null
          } satisfies ItemPrefill
        };
      }
      const detail: EquipmentItemDetail = await dr.json();
      return {
        role: it.role,
        prefill: {
          id: detail.id,
          display_name: detail.display_name,
          specs: detail.specs
        } satisfies ItemPrefill
      };
    })
  );

  const prefill: EditPrefill = {};
  for (const { role, prefill: item } of detailResults) {
    if (role === 'optical_tube') prefill.optical_tube = item;
    else if (role === 'main_camera') prefill.main_camera = item;
    else if (role === 'mount') prefill.mount = item;
    else if (role === 'focal_modifier') prefill.focal_modifier = item;
  }

  // Filters: expose as minimal chip array (filter_type/bandwidth loaded client-side if needed).
  prefill.filters = setup.items
    .filter((it) => it.role === 'filter')
    .map((it, i) => ({
      id: it.item.id,
      display_name: it.item.display_name,
      filter_type: null as null,
      bandwidth_nm: null as null,
      position: i
    }));

  return { setup, prefill };
};
