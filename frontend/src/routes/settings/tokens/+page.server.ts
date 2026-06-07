import type { PageServerLoad, Actions } from './$types';
import { fail } from '@sveltejs/kit';
import { api, ApiError } from '$lib/api/client';

export const load: PageServerLoad = async ({ fetch }) => ({
  tokens: await api.listApiTokens({ fetch })
});

export const actions: Actions = {
  create: async ({ request, fetch }) => {
    const fd = await request.formData();
    const name = String(fd.get('name') ?? '').trim();
    if (!name) return fail(400, { error: 'name_required' });
    try {
      const created = await api.createApiToken(name, { fetch });
      // The secret crosses to the page exactly once, in the action result.
      return { ok: true, created };
    } catch (e) {
      if (e instanceof ApiError && e.status === 400) return fail(400, { error: 'invalid' });
      return fail(500, { error: 'server' });
    }
  },
  revoke: async ({ request, fetch }) => {
    const fd = await request.formData();
    const id = String(fd.get('id') ?? '');
    if (!id) return fail(400, { error: 'missing_id' });
    try {
      await api.revokeApiToken(id, { fetch });
      return { ok: true };
    } catch {
      return fail(500, { error: 'server' });
    }
  }
};
