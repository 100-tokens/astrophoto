import { error, redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
import type { SetupSummary } from '$lib/api/SetupSummary';

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
