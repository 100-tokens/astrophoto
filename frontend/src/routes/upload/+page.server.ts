import { fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ locals }) => {
  if (!locals.user) redirect(303, '/signin');
  return {};
};

export const actions: Actions = {
  default: async ({ request, fetch, cookies }) => {
    const data = await request.formData();
    const file = data.get('file');
    if (!(file instanceof File) || file.size === 0) {
      return fail(400, { message: 'Choose a file to upload.' });
    }
    if (file.size > 50 * 1024 * 1024) {
      return fail(413, { message: 'File too large (max 50 MB).' });
    }
    const forwarded = new FormData();
    forwarded.append('file', file, file.name);
    const cookie = cookies.getAll().map((c) => `${c.name}=${c.value}`).join('; ');
    let res: Response;
    try {
      res = await fetch(`${API}/api/photos`, {
        method: 'POST', headers: { Cookie: cookie }, body: forwarded
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Network error';
      return fail(503, { message: `Backend unreachable: ${msg}` });
    }
    if (!res.ok) {
      if (res.status === 401) return fail(401, { message: 'Sign in required.' });
      const txt = await res.text();
      return fail(500, { message: `Upload failed: ${txt}` });
    }
    const body = (await res.json()) as { id: string; status: string };
    redirect(303, `/upload/${body.id}/verify`);
  }
};
