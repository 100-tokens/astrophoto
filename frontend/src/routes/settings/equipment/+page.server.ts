import { error, fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import type { SetupSummary } from '$lib/api/SetupSummary';
import type { SetupDetail } from '$lib/api/SetupDetail';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ locals, fetch, cookies }) => {
  if (!locals.user) redirect(303, '/signin');
  const cookie = cookies
    .getAll()
    .map((c) => `${c.name}=${c.value}`)
    .join('; ');
  const r = await fetch(`${API}/api/equipment/setups`, {
    headers: { Cookie: cookie }
  });
  if (!r.ok) error(500, 'Backend error');
  const setups: SetupSummary[] = await r.json();
  return { setups };
};

export const actions: Actions = {
  setDefault: async ({ request, fetch, cookies }) => {
    const fd = await request.formData();
    const id = fd.get('id');
    if (typeof id !== 'string' || !id) return fail(400, { error: 'Missing id' });

    const cookie = cookies
      .getAll()
      .map((c) => `${c.name}=${c.value}`)
      .join('; ');

    // Fetch the full detail so we can PATCH with replace-all items.
    const dr = await fetch(`${API}/api/equipment/setups/${id}`, {
      headers: { Cookie: cookie }
    });
    if (!dr.ok) return fail(dr.status, { error: 'Could not load setup' });
    const detail: SetupDetail = await dr.json();

    const body = {
      name: detail.name,
      description: detail.description,
      location: detail.location,
      is_remote: detail.is_remote,
      is_default: true,
      guiding: detail.guiding,
      items: detail.items.map((it) => ({ role: it.role, item_id: it.item.id }))
    };

    const r = await fetch(`${API}/api/equipment/setups/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json', Cookie: cookie },
      body: JSON.stringify(body)
    });
    if (!r.ok) return fail(r.status, { error: 'Could not set default' });
    redirect(303, '/settings/equipment');
  },

  delete: async ({ request, fetch, cookies }) => {
    const fd = await request.formData();
    const id = fd.get('id');
    if (typeof id !== 'string' || !id) return fail(400, { error: 'Missing id' });

    const cookie = cookies
      .getAll()
      .map((c) => `${c.name}=${c.value}`)
      .join('; ');

    const r = await fetch(`${API}/api/equipment/setups/${id}`, {
      method: 'DELETE',
      headers: { Cookie: cookie }
    });
    if (r.status !== 204 && !r.ok) return fail(r.status, { error: 'Could not delete setup' });
    redirect(303, '/settings/equipment');
  }
};
