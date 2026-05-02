import type { PageServerLoad, Actions } from './$types';
import { fail } from '@sveltejs/kit';
import { api } from '$lib/api/client';

export const load: PageServerLoad = async ({ locals }) => ({
  user: locals.user
});

export const actions: Actions = {
  requestChange: async ({ request, fetch }) => {
    const fd = await request.formData();
    const new_email = String(fd.get('new_email') ?? '').trim();
    const current_password = String(fd.get('current_password') ?? '');
    if (!new_email) return fail(400, { error: 'missing_email' });
    try {
      await api.requestEmailChange(new_email, current_password, { fetch });
      return { ok: true };
    } catch {
      return fail(401, { error: 'wrong_password' });
    }
  }
};
