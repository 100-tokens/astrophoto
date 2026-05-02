import { api, ApiError } from '$lib/api/client';
import type { Handle } from '@sveltejs/kit';

// The backend issues `__Host-session=` over HTTPS (prod) and the unprefixed
// `session=` over plain HTTP (dev); browsers reject the `__Host-` prefix
// without Secure. Match either to decide whether to call /api/auth/me.
const SESSION_COOKIE_RE = /(?:^|;\s*)(?:__Host-session|session)=/;

function parseCookie(header: string, name: string): string | null {
  const re = new RegExp('(?:^|;\\s*)' + name + '=([^;]+)');
  const m = header.match(re);
  return m ? decodeURIComponent(m[1] ?? '') : null;
}

export const handle: Handle = async ({ event, resolve }) => {
  const cookie = event.request.headers.get('cookie') ?? '';
  if (SESSION_COOKIE_RE.test(cookie)) {
    try {
      const user = await api.me({ fetch: event.fetch, cookie });
      event.locals.user = {
        id: user.id,
        email: user.email,
        displayName: user.display_name,
        following_ids: user.following_ids ?? [],
        pending_deletion_at: user.pending_deletion_at ?? null
      };
    } catch (e) {
      if (e instanceof ApiError && e.status === 401) {
        event.locals.user = null;
      } else {
        // Don't break the page on transient backend errors.
        event.locals.user = null;
      }
    }
  } else {
    event.locals.user = null;
  }

  const themeCookie = parseCookie(cookie, 'theme');
  const densityCookie = parseCookie(cookie, 'density');
  const theme = (themeCookie === 'light' ? 'light' : 'dark') as 'dark' | 'light';
  const density = (densityCookie === 'data' ? 'data' : 'work') as 'work' | 'data';
  event.locals.preferences = { theme, density };

  return resolve(event, {
    // Global replace + done = true filter so the placeholder is robust to
    // chunk boundaries (the SvelteKit `transformPageChunk` is called per
    // chunk; a non-global `replace` would silently miss multi-occurrence).
    transformPageChunk: ({ html }) => html.replace(/%theme%/g, theme).replace(/%density%/g, density)
  });
};
