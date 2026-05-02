import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

const API = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ params, fetch }) => {
  let res: Response;
  try {
    res = await fetch(`${API}/api/auth/email-change/confirm`, {
      method: 'POST',
      credentials: 'include',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token: params.token })
    });
  } catch {
    return { status: 'error' as const };
  }

  let body: { status: 'success' | 'expired' | 'taken' };
  try {
    body = (await res.json()) as { status: 'success' | 'expired' | 'taken' };
  } catch {
    return { status: 'error' as const };
  }

  if (body.status === 'success') {
    throw redirect(303, '/settings/email?changed=1');
  }
  return { status: body.status };
};
