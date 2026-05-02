import type { Actions } from './$types';
import { fail } from '@sveltejs/kit';
import { api } from '$lib/api/client';

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
    } catch {
      return fail(401, { error: 'wrong_password' });
    }
  }
};
