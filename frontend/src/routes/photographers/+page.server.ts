import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

// /photographers is in the header nav per the design but no dedicated screen
// exists in the showcase spec yet. Redirect to /explore which is the closest
// match — a feed of all published frames across photographers. When a real
// /photographers index page (leaderboards, alphabetical list, etc.) is built,
// drop this redirect.
export const load: PageServerLoad = async () => {
  redirect(307, '/explore');
};
