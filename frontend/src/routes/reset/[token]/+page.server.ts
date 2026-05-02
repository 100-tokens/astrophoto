import { fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

const API = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ params }) => {
  // No token-validate endpoint — render the form optimistically.
  // Expired-or-used state is handled at submit time.
  return { token: params.token };
};

export const actions: Actions = {
  default: async ({ request, params, fetch, cookies }) => {
    const fd = await request.formData();
    const new_password = String(fd.get('new_password') ?? '');
    if (new_password.length < 12) return fail(400, { error: 'too_short' as const });

    let res: Response;
    try {
      res = await fetch(`${API}/api/auth/password-reset/confirm`, {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ token: params.token, new_password })
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Network error.';
      return fail(503, { error: 'server' as const, detail: msg });
    }

    if (res.status === 410) return fail(410, { error: 'expired_or_used' as const });
    if (res.status === 400) {
      // Backend returns 400 with a code like "password_too_short" or
      // "password_too_common" when the strength validator rejects.
      const body = (await res.json().catch(() => ({}))) as { code?: string };
      const code = body.code ?? 'invalid';
      return fail(400, { error: 'weak' as const, detail: code });
    }
    if (!res.ok) return fail(500, { error: 'server' as const });

    // Forward the session cookie the backend sets after auto-login.
    const setCookie = res.headers.get('set-cookie');
    if (setCookie) {
      const parts = setCookie.split(';').map((s) => s.trim());
      const pair = parts[0] ?? '';
      const attrs = parts.slice(1);
      const eq = pair.indexOf('=');
      const name = pair.slice(0, eq);
      const value = pair.slice(eq + 1);
      const opts: {
        path: string;
        httpOnly?: boolean;
        secure?: boolean;
        sameSite?: 'lax' | 'strict' | 'none';
        maxAge?: number;
      } = { path: '/' };
      for (const a of attrs) {
        const eqIdx = a.indexOf('=');
        const k = eqIdx === -1 ? a : a.slice(0, eqIdx);
        const v = eqIdx === -1 ? undefined : a.slice(eqIdx + 1);
        const kl = k.toLowerCase();
        if (kl === 'path' && v) opts.path = v;
        else if (kl === 'samesite' && v)
          opts.sameSite = v.toLowerCase() as 'lax' | 'strict' | 'none';
        else if (kl === 'httponly') opts.httpOnly = true;
        else if (kl === 'secure') opts.secure = true;
        else if (kl === 'max-age' && v) opts.maxAge = parseInt(v, 10);
      }
      cookies.set(name, value, opts);
    }

    throw redirect(303, '/');
  }
};
