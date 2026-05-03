import type { PageServerLoad } from './$types';
import { error, redirect } from '@sveltejs/kit';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ params, fetch }) => {
  // The backend returns 308 Permanent Redirect with Location: /u/<handle>/p/<short_id>.
  // We use redirect:'manual' so SvelteKit's server-side fetch does not auto-follow,
  // then re-issue the canonical URL as a 301 to the browser.
  const r = await fetch(`${API}/api/photos/by-uuid/${params.slug}`, { redirect: 'manual' });

  if (r.status === 308 || r.status === 301) {
    const loc = r.headers.get('location');
    if (loc) throw redirect(301, loc);
  }

  // Defensive: if the backend ever returns 200 JSON with handle+short_id, use that.
  if (r.ok) {
    const body = (await r.json()) as { handle?: string; short_id?: string };
    if (body.handle && body.short_id) {
      throw redirect(301, `/u/${body.handle}/p/${body.short_id}`);
    }
  }

  throw error(404, 'Photo not found');
};
