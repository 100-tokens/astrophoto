import type { PageServerLoad, Actions } from './$types';
import { api } from '$lib/api/client';

export const load: PageServerLoad = async ({ fetch }) => ({
  profile: await api.getProfile({ fetch })
});

export const actions: Actions = {
  default: async ({ request, fetch }) => {
    const fd = await request.formData();
    await api.putProfile({ display_name: String(fd.get('display_name') ?? '') }, { fetch });
    return { ok: true };
  }
};
