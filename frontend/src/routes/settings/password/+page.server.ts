import type { Actions } from './$types';
import { fail } from '@sveltejs/kit';

const API = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080';

export const actions: Actions = {
  default: async ({ request, fetch, cookies }) => {
    const fd = await request.formData();
    const current_password_raw = String(fd.get('current_password') ?? '');
    const new_password = String(fd.get('new_password') ?? '');
    if (new_password.length < 12) return fail(400, { error: 'too_short' });
    const body =
      current_password_raw.length > 0
        ? { current_password: current_password_raw, new_password }
        : { new_password };

    let res: Response;
    try {
      res = await fetch(`${API}/api/me/password-change`, {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body)
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Network error.';
      return fail(503, { error: 'server', detail: msg });
    }

    if (!res.ok) {
      if (res.status === 401) return fail(401, { error: 'wrong_password' });
      if (res.status === 429) return fail(429, { error: 'throttled' });
      if (res.status === 400) {
        let msg = '';
        try {
          const b = (await res.json()) as { message?: string };
          msg = b.message ?? '';
        } catch {
          /* ignore */
        }
        if (msg.includes('password_too_short')) return fail(400, { error: 'too_short' });
        if (msg.includes('password_too_common')) return fail(400, { error: 'too_common' });
        return fail(400, { error: 'invalid' });
      }
      return fail(500, { error: 'server' });
    }

    // Forward the rotated session cookie the backend issues on password change.
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

    return { ok: true };
  }
};
