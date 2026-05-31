import type { PageServerLoad } from './$types';
import { fetchSettings } from '$lib/api/adminClient';
import { error } from '@sveltejs/kit';

export const load: PageServerLoad = async ({ fetch }) => {
  try {
    return { settings: await fetchSettings(fetch) };
  } catch (_e) {
    throw error(500, 'Failed to load settings');
  }
};
