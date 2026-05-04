import { redirect } from '@sveltejs/kit';
import type { PageServerLoad, Actions } from './$types';
import { api, ApiError } from '$lib/api/client';

export const load: PageServerLoad = async ({ locals }) => ({
  preferences: locals.preferences
});

const COOKIE_OPTS = {
  path: '/',
  maxAge: 60 * 60 * 24 * 365,
  sameSite: 'lax' as const,
  httpOnly: false
};

async function syncToBackend(
  fetch: typeof globalThis.fetch,
  body: { theme?: string; density?: string }
): Promise<void> {
  try {
    await api.putPreferences(body, { fetch });
  } catch (e) {
    // Anonymous visitors can still pick a theme; only persist server-side
    // when authenticated. Other failures are non-fatal for a UI toggle.
    if (!(e instanceof ApiError && e.status === 401)) throw e;
  }
}

export const actions: Actions = {
  setTheme: async ({ request, fetch, cookies }) => {
    const fd = await request.formData();
    const theme = String(fd.get('theme') ?? 'dark');
    cookies.set('theme', theme, COOKIE_OPTS);
    await syncToBackend(fetch, { theme });
    throw redirect(303, '/settings/appearance');
  },
  setDensity: async ({ request, fetch, cookies }) => {
    const fd = await request.formData();
    const density = String(fd.get('density') ?? 'work');
    cookies.set('density', density, COOKIE_OPTS);
    await syncToBackend(fetch, { density });
    throw redirect(303, '/settings/appearance');
  }
};
