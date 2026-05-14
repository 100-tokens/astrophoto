import { fail } from '@sveltejs/kit';
import type { Actions } from './$types';

const API = process.env.BACKEND_URL ?? import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080';

export const actions: Actions = {
  resend: async ({ request, fetch }) => {
    const fd = await request.formData();
    const email = String(fd.get('email') ?? '').trim();
    if (!email) return fail(400, { error: 'missing_email' as const });

    try {
      // Backend always returns 204 No Content (anti-enumeration).
      await fetch(`${API}/api/auth/resend-verification`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email })
      });
    } catch {
      // Best-effort — frontend never reveals whether the address exists.
    }
    return { ok: true as const };
  }
};
