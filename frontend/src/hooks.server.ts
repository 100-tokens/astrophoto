import { api, ApiError } from '$lib/api/client';
import type { Handle } from '@sveltejs/kit';

// The backend issues `__Host-session=` over HTTPS (prod) and the unprefixed
// `session=` over plain HTTP (dev); browsers reject the `__Host-` prefix
// without Secure. Match either to decide whether to call /api/auth/me.
const SESSION_COOKIE_RE = /(?:^|;\s*)(?:__Host-session|session)=/;

export const handle: Handle = async ({ event, resolve }) => {
  const cookie = event.request.headers.get('cookie') ?? '';
  if (SESSION_COOKIE_RE.test(cookie)) {
    try {
      const user = await api.me({ fetch: event.fetch, cookie });
      event.locals.user = {
        id: user.id,
        displayName: user.display_name,
        following_ids: user.following_ids ?? []
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
  return resolve(event);
};
