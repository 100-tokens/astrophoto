import type { PageServerLoad, Actions } from './$types';
import { fail } from '@sveltejs/kit';
import { api, ApiError } from '$lib/api/client';

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
    } catch (e) {
      if (e instanceof ApiError) {
        if (e.status === 401) return fail(401, { error: 'wrong_password' });
        if (e.status === 429) return fail(429, { error: 'throttled' });
        if (e.status === 400) {
          const body = e.body as { message?: string } | undefined;
          const msg = body?.message ?? '';
          if (msg.includes('same_email')) return fail(400, { error: 'same_email' });
          if (msg.includes('invalid_email')) return fail(400, { error: 'invalid_email' });
          return fail(400, { error: 'invalid' });
        }
      }
      return fail(500, { error: 'server' });
    }
  }
};
