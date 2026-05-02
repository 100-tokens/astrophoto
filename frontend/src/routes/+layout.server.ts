import type { LayoutServerLoad } from './$types';
import { api } from '$lib/api/client';

export const load: LayoutServerLoad = async ({ locals, fetch }) => {
  let frame_count: number | null = null;
  if (locals.user?.pending_deletion_at) {
    try {
      const v = await api.photosCount({ fetch });
      frame_count = v.count;
    } catch {
      // Non-critical: grace banner shows without count on fetch failure.
    }
  }
  return { user: locals.user, frame_count };
};
