import { error, fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';
import type { SetupDetail } from '$lib/api/SetupDetail';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

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
  return { setup };
};

type ItemRow = { id: string; display_name: string };

async function resolveItem(
  fetchFn: typeof globalThis.fetch,
  cookie: string,
  kind: string,
  displayName: string
): Promise<string | null> {
  const t = displayName.trim();
  if (!t) return null;
  const r = await fetchFn(`${API}/api/equipment/items`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Cookie: cookie },
    body: JSON.stringify({ kind, display_name: t })
  });
  if (!r.ok) return null;
  const row: ItemRow = await r.json();
  return row.id;
}

export const actions: Actions = {
  default: async ({ request, params, fetch, cookies }) => {
    const fd = await request.formData();
    const cookie = cookies
      .getAll()
      .map((c) => `${c.name}=${c.value}`)
      .join('; ');

    const name = (fd.get('name') as string | null)?.trim() ?? '';
    if (!name) return fail(422, { error: 'Name is required' });

    const strOrNull = (k: string): string | null => {
      const v = fd.get(k);
      return typeof v === 'string' && v.trim() !== '' ? v.trim() : null;
    };

    const items: { role: string; item_id: string }[] = [];

    const roles: [string, string, string][] = [
      ['optical_tube', 'telescope', 'optical_tube_text'],
      ['focal_modifier', 'focal_modifier', 'focal_modifier_text'],
      ['main_camera', 'camera', 'main_camera_text'],
      ['mount', 'mount', 'mount_text']
    ];

    for (const [role, kind, field] of roles) {
      const text = strOrNull(field);
      if (text) {
        const id = await resolveItem(fetch, cookie, kind, text);
        if (!id) return fail(422, { error: `Could not resolve ${kind} "${text}"` });
        items.push({ role, item_id: id });
      }
    }

    for (const filterText of fd.getAll('filter_text')) {
      if (typeof filterText === 'string' && filterText.trim()) {
        const id = await resolveItem(fetch, cookie, 'filter', filterText);
        if (!id) return fail(422, { error: `Could not resolve filter "${filterText}"` });
        items.push({ role: 'filter', item_id: id });
      }
    }

    const body = {
      name,
      description: strOrNull('description'),
      location: strOrNull('location'),
      is_remote: fd.get('is_remote') === 'on',
      is_default: fd.get('is_default') === 'on',
      guiding: strOrNull('guiding'),
      items
    };

    const r = await fetch(`${API}/api/equipment/setups/${params.id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json', Cookie: cookie },
      body: JSON.stringify(body)
    });

    if (r.status === 422) {
      try {
        const b: unknown = await r.json();
        const msg =
          typeof b === 'object' && b !== null && 'error' in b && typeof b.error === 'string'
            ? b.error
            : 'Validation error';
        return fail(422, { error: msg });
      } catch {
        return fail(422, { error: 'Validation error' });
      }
    }
    if (r.status === 404) return fail(404, { error: 'Setup no longer exists' });
    if (!r.ok) return fail(r.status, { error: `Backend error (${r.status})` });

    redirect(303, '/settings/equipment');
  }
};
