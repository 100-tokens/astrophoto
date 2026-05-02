import { redirect } from '@sveltejs/kit';
import type { PageServerLoad, Actions } from './$types';
import { api } from '$lib/api/client';

export const load: PageServerLoad = async ({ locals }) => ({
  preferences: locals.preferences
});

export const actions: Actions = {
  setTheme: async ({ request, fetch, cookies }) => {
    const fd = await request.formData();
    const theme = String(fd.get('theme') ?? 'dark');
    cookies.set('theme', theme, { path: '/', maxAge: 60 * 60 * 24 * 365, sameSite: 'lax' });
    await api.putPreferences({ theme }, { fetch });
    throw redirect(303, '/settings/appearance');
  },
  setDensity: async ({ request, fetch, cookies }) => {
    const fd = await request.formData();
    const density = String(fd.get('density') ?? 'work');
    cookies.set('density', density, { path: '/', maxAge: 60 * 60 * 24 * 365, sameSite: 'lax' });
    await api.putPreferences({ density }, { fetch });
    throw redirect(303, '/settings/appearance');
  }
};
