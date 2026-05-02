import type { PageServerLoad, Actions } from './$types';
import { api } from '$lib/api/client';

export const load: PageServerLoad = async ({ fetch }) => ({
  sessions: await api.listSessions({ fetch })
});

export const actions: Actions = {
  revoke: async ({ request, fetch }) => {
    const fd = await request.formData();
    const id = String(fd.get('id') ?? '');
    await api.revokeSession(id, { fetch });
    return { ok: true };
  },
  signOutOthers: async ({ fetch }) => {
    await api.signOutOthers({ fetch });
    return { ok: true };
  }
};
