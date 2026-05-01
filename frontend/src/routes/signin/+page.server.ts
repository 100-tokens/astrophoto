import { fail, redirect } from '@sveltejs/kit';
import type { Actions } from './$types';

const API = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080';

export const actions: Actions = {
  default: async ({ request, fetch, cookies }) => {
    const data = await request.formData();
    const email = String(data.get('email') ?? '');
    const password = String(data.get('password') ?? '');

    if (!email || !password) {
      return fail(400, { email, message: 'Email and password are required.' });
    }

    let res: Response;
    try {
      res = await fetch(`${API}/api/auth/login`, {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, password })
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Network error.';
      return fail(503, { email, message: `Backend unreachable: ${msg}` });
    }

    if (!res.ok) {
      if (res.status === 401) {
        return fail(401, { email, message: 'Invalid email or password.' });
      }
      const txt = await res.text();
      return fail(500, { email, message: `Sign-in failed: ${txt}` });
    }

    // Forward the session cookie from the backend to the browser.
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
