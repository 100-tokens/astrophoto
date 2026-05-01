import { fail } from '@sveltejs/kit';
import type { Actions } from './$types';

export const actions: Actions = {
  default: async ({ request }) => {
    const data = await request.formData();
    const email = String(data.get('email') ?? '');
    const displayName = String(data.get('display_name') ?? '');
    // Auth backend is Phase 4 (not yet implemented).
    return fail(501, {
      email,
      displayName,
      message: 'Account creation is not wired to the backend yet. Coming in Phase 4.'
    });
  }
};
