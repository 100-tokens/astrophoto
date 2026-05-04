import { error, redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import type { SetupDetail } from '$lib/api/SetupDetail';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ params, locals, fetch, cookies }) => {
  if (!locals.user) redirect(303, '/signin');
  const cookie = cookies.getAll().map((c) => `${c.name}=${c.value}`).join('; ');
  const r = await fetch(`${API}/api/equipment/setups/${params.id}`, {
    headers: { Cookie: cookie }
  });
  if (r.status === 404) error(404, 'Setup not found');
  if (!r.ok) error(500, 'Backend error');
  const setup: SetupDetail = await r.json();
  return { setup };
};
