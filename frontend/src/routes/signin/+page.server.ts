import { fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

// Resolve the public backend origin at server-runtime. Koyeb staging sets
// BACKEND_URL but build-time VITE_API_BASE_URL isn't visible to client
// bundles, so the form action below needs the URL plumbed through PageData.
const API = process.env.BACKEND_URL ?? import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ locals }) => {
  if (locals.user) throw redirect(303, '/');
  return {
    // Relative path → goes through the SvelteKit /api proxy so the session
    // cookie set by the OAuth callback lands on the *frontend* origin. If
    // this ever points at api.<domain> directly, the __Host-session cookie
    // ends up scoped to the API host and the frontend can never see it.
    googleOauthUrl: '/api/auth/oauth/google/start'
  };
};

export const actions: Actions = {
  default: async ({ request, fetch, cookies, getClientAddress }) => {
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
        headers: {
          'Content-Type': 'application/json',
          'User-Agent': request.headers.get('user-agent') ?? '',
          'X-Forwarded-For': getClientAddress()
        },
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
      if (res.status === 403) {
        // Backend rejects sign-in for users with email_verified_at IS NULL.
        // Push the user to the check-email page to resend or wait for the link.
        throw redirect(303, `/signup/check-email?email=${encodeURIComponent(email)}`);
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
