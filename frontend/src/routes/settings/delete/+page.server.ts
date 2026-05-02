import type { PageServerLoad, Actions } from './$types';
import { fail, redirect } from '@sveltejs/kit';
import { api, ApiError } from '$lib/api/client';

export const load: PageServerLoad = async ({ locals }) => ({
  pending_deletion_at: locals.user?.pending_deletion_at ?? null
});

export const actions: Actions = {
  request: async ({ request, fetch }) => {
    const fd = await request.formData();
    const phrase = String(fd.get('confirmation_phrase') ?? '');
    const current_password_raw = String(fd.get('current_password') ?? '');
    if (phrase !== 'DELETE MY ACCOUNT') return fail(400, { error: 'phrase' });
    const body =
      current_password_raw.length > 0
        ? { confirmation_phrase: phrase, current_password: current_password_raw }
        : { confirmation_phrase: phrase };
    try {
      await api.requestDeletion(body, { fetch });
    } catch (e) {
      if (e instanceof ApiError) {
        if (e.status === 401) return fail(401, { error: 'wrong_password' });
        if (e.status === 429) return fail(429, { error: 'throttled' });
        if (e.status === 400) {
          const errorBody = e.body as { message?: string } | undefined;
          const msg = errorBody?.message ?? '';
          if (msg.includes('phrase_mismatch')) return fail(400, { error: 'phrase' });
          return fail(400, { error: 'invalid' });
        }
      }
      return fail(500, { error: 'server' });
    }
    redirect(303, '/settings/delete');
  },
  cancel: async ({ fetch }) => {
    await api.cancelDeletion({ fetch });
    redirect(303, '/settings/delete');
  }
};
