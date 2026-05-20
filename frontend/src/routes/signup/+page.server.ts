import { fail, redirect } from '@sveltejs/kit';
import type { Actions, PageServerLoad } from './$types';

// process.env.BACKEND_URL is what Koyeb sets at runtime;
// import.meta.env.VITE_API_BASE_URL is the historical name used elsewhere.
// Keep both so neither env breaks.
const API = process.env.BACKEND_URL ?? import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ locals }) => {
  // Already authenticated → no need to see the signup form.
  if (locals.user) throw redirect(303, '/');
  return {
    // Relative — see signin/+page.server.ts for why. Routing through the
    // SvelteKit /api proxy is what scopes the session cookie to the
    // frontend origin.
    googleOauthUrl: '/api/auth/oauth/google/start'
  };
};

export const actions: Actions = {
  default: async ({ request, fetch, cookies, getClientAddress }) => {
    const data = await request.formData();
    const email = String(data.get('email') ?? '');
    const password = String(data.get('password') ?? '');
    const display_name = String(data.get('display_name') ?? '');
    const handle = String(data.get('handle') ?? '').trim();

    if (!email || !password || !display_name || !handle) {
      return fail(400, { email, display_name, handle, message: 'All fields are required.' });
    }
    if (password.length < 10) {
      return fail(400, {
        email,
        display_name,
        handle,
        message: 'Password must be at least 10 characters.'
      });
    }

    let res: Response;
    try {
      res = await fetch(`${API}/api/auth/signup`, {
        method: 'POST',
        credentials: 'include',
        headers: {
          'Content-Type': 'application/json',
          'User-Agent': request.headers.get('user-agent') ?? '',
          'X-Forwarded-For': getClientAddress()
        },
        body: JSON.stringify({ email, password, display_name, handle })
      });
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'Network error.';
      return fail(503, { email, display_name, handle, message: `Backend unreachable: ${msg}` });
    }

    if (!res.ok) {
      if (res.status === 409) {
        // Backend returns {"error":"conflict","message":"conflict: handle already taken"}
        // or {"error":"conflict","message":"conflict: email already in use"}.
        // Disambiguate by inspecting the message text.
        const body = (await res.json().catch(() => ({}))) as { message?: unknown };
        const msg = String(body.message ?? '');
        if (msg.includes('handle')) {
          return fail(409, {
            email,
            display_name,
            handle,
            handleError: 'That handle is already taken.'
          });
        }
        return fail(409, {
          email,
          display_name,
          handle,
          message: 'An account with that email already exists.'
        });
      }
      if (res.status === 422) {
        return fail(422, { email, display_name, handle, message: 'Please check your inputs.' });
      }
      const txt = await res.text();
      return fail(500, { email, display_name, handle, message: `Sign-up failed: ${txt}` });
    }

    // Backend now returns 202 Accepted with { status: 'verification_required', email }.
    // No cookie is set — the user must click the email link to finish signup.
    throw redirect(303, `/signup/check-email?email=${encodeURIComponent(email)}`);
  }
};
