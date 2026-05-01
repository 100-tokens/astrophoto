import { fail, redirect } from '@sveltejs/kit';
import type { Actions } from './$types';

const API = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080';

export const actions: Actions = {
  default: async ({ request, fetch, cookies }) => {
    const data = await request.formData();
    const email = String(data.get('email') ?? '');
    const password = String(data.get('password') ?? '');
    const display_name = String(data.get('display_name') ?? '');

    if (!email || !password || !display_name) {
      return fail(400, { email, display_name, message: 'All fields are required.' });
    }
    if (password.length < 10) {
      return fail(400, {
        email,
        display_name,
        message: 'Password must be at least 10 characters.'
      });
    }

    let res: Response;
    try {
      res = await fetch(`${API}/api/auth/signup`, {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, password, display_name })
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Network error.';
      return fail(503, { email, display_name, message: `Backend unreachable: ${msg}` });
    }

    if (!res.ok) {
      if (res.status === 409) {
        return fail(409, {
          email,
          display_name,
          message: 'An account with that email already exists.'
        });
      }
      if (res.status === 422) {
        return fail(422, { email, display_name, message: 'Please check your inputs.' });
      }
      const txt = await res.text();
      return fail(500, { email, display_name, message: `Sign-up failed: ${txt}` });
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
