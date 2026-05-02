import type { Actions } from './$types';
import { fail } from '@sveltejs/kit';
import { api, ApiError } from '$lib/api/client';

export const actions: Actions = {
  default: async ({ request, fetch }) => {
    const fd = await request.formData();
    const current_password_raw = String(fd.get('current_password') ?? '');
    const new_password = String(fd.get('new_password') ?? '');
    if (new_password.length < 12) return fail(400, { error: 'too_short' });
    const body =
      current_password_raw.length > 0
        ? { current_password: current_password_raw, new_password }
        : { new_password };
    try {
      await api.changePassword(body, { fetch });
      return { ok: true };
    } catch (e) {
      if (e instanceof ApiError) {
        if (e.status === 401) return fail(401, { error: 'wrong_password' });
        if (e.status === 429) return fail(429, { error: 'throttled' });
        if (e.status === 400) {
          const errorBody = e.body as { message?: string } | undefined;
          const msg = errorBody?.message ?? '';
          if (msg.includes('password_too_short')) return fail(400, { error: 'too_short' });
          if (msg.includes('password_too_common')) return fail(400, { error: 'too_common' });
          return fail(400, { error: 'invalid' });
        }
      }
      return fail(500, { error: 'server' });
    }
  }
};
