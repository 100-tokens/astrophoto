import { error, redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import type { SetupDetail } from '$lib/api/SetupDetail';
import type { EquipmentItemDetail } from '$lib/api/EquipmentItemDetail';
import type { EquipmentSpecsPayload } from '$lib/api/EquipmentSpecsPayload';
import type { FilterType } from '$lib/api/FilterType';

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
    filter_type: FilterType | null;
    bandwidth_nm: number | null;
    position: number;
  }>;
};

const NON_FILTER_ROLES = ['optical_tube', 'main_camera', 'mount', 'focal_modifier'] as const;

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

  // Fetch full item details (with specs) for all roles in parallel.
  const allResults = await Promise.all(
    setup.items.map(async (it) => {
      const dr = await fetch(`${API}/api/equipment/items/${it.item.id}`, {
        headers: { Cookie: cookie }
      });
      if (!dr.ok) {
        return { role: it.role, detail: null as EquipmentItemDetail | null, item: it.item };
      }
      const detail: EquipmentItemDetail = await dr.json();
      return { role: it.role, detail, item: it.item };
    })
  );

  const prefill: EditPrefill = {};

  // Non-filter roles — build ItemPrefill entries
  for (const { role, detail, item } of allResults) {
    if (!(NON_FILTER_ROLES as readonly string[]).includes(role)) continue;
    const entry: ItemPrefill = {
      id: detail?.id ?? item.id,
      display_name: detail?.display_name ?? item.display_name,
      specs: detail?.specs ?? null
    };
    if (role === 'optical_tube') prefill.optical_tube = entry;
    else if (role === 'main_camera') prefill.main_camera = entry;
    else if (role === 'mount') prefill.mount = entry;
    else if (role === 'focal_modifier') prefill.focal_modifier = entry;
  }

  // Filters — extract filter_type and bandwidth_nm from specs when available
  let filterPosition = 0;
  prefill.filters = allResults
    .filter(({ role }) => role === 'filter')
    .map(({ detail, item }) => {
      const specs = detail?.specs;
      let filter_type: FilterType | null = null;
      let bandwidth_nm: number | null = null;
      if (specs && specs.kind === 'filter') {
        filter_type = specs.filter_type;
        bandwidth_nm = specs.bandwidth_nm;
      }
      return {
        id: detail?.id ?? item.id,
        display_name: detail?.display_name ?? item.display_name,
        filter_type,
        bandwidth_nm,
        position: filterPosition++
      };
    });

  return { setup, prefill };
};
