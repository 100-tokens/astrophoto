import { redirect } from '@sveltejs/kit';
import type { LayoutServerLoad } from './$types';

// Server-side guard for the whole /admin section. Unauthenticated users are
// sent to sign in; authenticated non-admins are bounced home (no leak that
// /admin even exists). The backend re-checks on every /api/admin/* call, so
// this guard is UX, not the security boundary.
export const load: LayoutServerLoad = async ({ locals, url }) => {
  if (!locals.user) {
    redirect(303, `/signin?next=${encodeURIComponent(url.pathname)}`);
  }
  if (!locals.user.isAdmin) {
    redirect(303, '/');
  }
  return { user: locals.user };
};
