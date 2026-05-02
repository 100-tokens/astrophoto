import { redirect } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

const API = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080';

export const POST: RequestHandler = async ({ fetch, cookies }) => {
  const cookie = cookies
    .getAll()
    .map((c) => `${c.name}=${c.value}`)
    .join('; ');
  try {
    await fetch(`${API}/api/auth/logout`, {
      method: 'POST',
      headers: { Cookie: cookie }
    });
  } catch {
    // Backend unreachable — fall through, clear cookies locally anyway.
  }
  // SvelteKit forbids set-cookie via setHeaders; use cookies.delete().
  // We clear both the dev-mode and prod-mode cookie names to be safe.
  cookies.delete('session', { path: '/' });
  cookies.delete('__Host-session', { path: '/' });
  throw redirect(303, '/');
};
