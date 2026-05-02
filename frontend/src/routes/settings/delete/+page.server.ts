import type { PageServerLoad, Actions } from './$types';
import { fail, redirect } from '@sveltejs/kit';
import { api } from '$lib/api/client';

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
    } catch {
      return fail(401, { error: 'wrong_password' });
    }
    redirect(303, '/settings/delete');
  },
  cancel: async ({ fetch }) => {
    await api.cancelDeletion({ fetch });
    redirect(303, '/settings/delete');
  }
};
