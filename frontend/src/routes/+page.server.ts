import { api } from '$lib/api/client';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch }) => {
  try {
    const health = await api.health(fetch);
    return { health };
  } catch {
    return { health: null };
  }
};
