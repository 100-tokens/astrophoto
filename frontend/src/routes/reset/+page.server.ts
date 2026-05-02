import { fail, redirect } from '@sveltejs/kit';
import type { Actions } from './$types';

const API = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080';

export const actions: Actions = {
  default: async ({ request, fetch }) => {
    const fd = await request.formData();
    const email = String(fd.get('email') ?? '').trim();
    if (!email) return fail(400, { error: 'missing_email' });

    try {
      await fetch(`${API}/api/auth/password-reset/request`, {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email })
      });
    } catch {
      // Intentionally swallow — we never reveal whether the email exists.
    }

    throw redirect(303, `/reset/sent?email=${encodeURIComponent(email)}`);
  }
};
