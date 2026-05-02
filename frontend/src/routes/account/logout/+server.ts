import { redirect } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

const API = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080';

export const POST: RequestHandler = async ({ fetch, cookies, setHeaders }) => {
  const cookie = cookies
    .getAll()
    .map((c) => `${c.name}=${c.value}`)
    .join('; ');
  let res: Response;
  try {
    res = await fetch(`${API}/api/auth/logout`, {
      method: 'POST',
      headers: { Cookie: cookie }
    });
  } catch {
    // Backend unreachable — clear client state anyway by setting an
    // expired session cookie locally (matches the dev-mode cookie name).
    setHeaders({
      'set-cookie':
        'session=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0'
    });
    throw redirect(303, '/');
  }
  const setCookie = res.headers.get('set-cookie');
  if (setCookie) setHeaders({ 'set-cookie': setCookie });
  throw redirect(303, '/');
};
