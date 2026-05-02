import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async ({ locals, fetch }) => {
  let frame_count: number | null = null;
  if (locals.user?.pending_deletion_at) {
    try {
      const r = await fetch('/api/me/photos/count');
      if (r.ok) {
        const v = await r.json();
        frame_count = typeof v.count === 'number' ? v.count : null;
      }
    } catch {
      // Non-critical: grace banner shows without count on fetch failure.
    }
  }
  return { user: locals.user, frame_count };
};
