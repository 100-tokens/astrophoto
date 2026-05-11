import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

// /drafts is the friendly URL the upload-page footer "Save & finish later"
// link points to. The canonical surface lives at /me/drafts because it
// requires auth and shares the /me account-scope tree.
export const load: PageServerLoad = async () => {
  redirect(307, '/me/drafts');
};
