import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

const API =
  process.env.BACKEND_URL ?? import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ params, fetch, cookies }) => {
  let res: Response;
  try {
    res = await fetch(`${API}/api/auth/verify-email`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token: params.token })
    });
  } catch {
    throw redirect(303, '/signup/check-email?expired=1');
  }

  if (res.status === 410) {
    throw redirect(303, '/signup/check-email?expired=1');
  }
  if (!res.ok) {
    throw redirect(303, '/signup/check-email?expired=1');
  }

  // Forward the session cookie that the backend set on auto-login.
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
};
