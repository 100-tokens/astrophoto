import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

// /admin lands on the equipment manager.
export const load: PageServerLoad = () => {
  redirect(307, '/admin/equipment');
};
